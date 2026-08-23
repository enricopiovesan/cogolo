//! MCP tool surfaces for bounded parallel proposal scheduling and execution
//! (spec `110-bounded-parallel-workflow-scheduling`, P2, ADR-0042).
//!
//! Extends `tools::proposals`' P1 lifecycle with a schedule-aware variant:
//! the same `WorkflowProposal` wire format, cross-validated the same way,
//! but levelized into concurrency waves and authorized against the
//! `pure_read`-only constraint (FR-004a) before execution. Mirrors the same
//! plain, fully-tested-function pattern as `tools::proposals` and
//! `tools::capabilities` rather than wiring into `stdio_server.rs`.

use ed25519_dalek::VerifyingKey;
use serde::Serialize;
use std::collections::HashMap;

use traverse_contracts::{
    ParallelScheduleLimits, ProposalLimits, SnapshotDigests, canonicalize_proposal,
    compute_parallel_schedule, proposal_snapshot_digest,
};
use traverse_registry::{ApplicationBundleManifest, CapabilityRegistry};
use traverse_runtime::parallel_proposal::{
    ParallelExecutionLimits, enforce_pure_read_only_parallelism, execute_parallel_proposal,
};
use traverse_runtime::proposal::{
    ApprovalTokenStore, QuotaLimits, QuotaTracker, validate_proposal_against_host_state,
};
use traverse_runtime::{LocalExecutor, Runtime};

use crate::McpError;
use crate::tools::proposals::{
    AuthorizeAndReserveQuotaRequest, ProposalDenial, ProposalExecutionResponse, cross_denial,
    parse_and_digest, structural_denial,
};

#[derive(Debug, Clone, Serialize)]
pub struct ParallelScheduleResponse {
    pub proposal_id: String,
    pub proposal_digest: String,
    pub valid: bool,
    pub errors: Vec<ProposalDenial>,
    /// Concurrency waves in dependency order; empty unless `valid` is true.
    pub waves: Vec<Vec<String>>,
    pub automatic_eligible: bool,
}

/// Validates a proposal (spec 109, unchanged) and, if valid, levelizes it
/// into concurrency waves and checks the FR-004a `pure_read`-only
/// constraint. A structural, cross-validation, schedule-bound, or
/// concurrent-side-effect denial is a normal, structured response, never an
/// [`McpError`] — matching FR-010's stable denial-code contract.
///
/// # Errors
///
/// Returns [`McpError`] only when `proposal_json` is not valid JSON.
pub fn compute_schedule_for_proposal(
    proposal_json: &str,
    manifest: &ApplicationBundleManifest,
    registry: &CapabilityRegistry,
    proposal_limits: &ProposalLimits,
    schedule_limits: &ParallelScheduleLimits,
) -> Result<ParallelScheduleResponse, McpError> {
    let (proposal, proposal_id, digest) = parse_and_digest(proposal_json)?;

    let canonical = match canonicalize_proposal(proposal, proposal_limits) {
        Ok(canonical) => canonical,
        Err(failure) => {
            return Ok(ParallelScheduleResponse {
                proposal_id,
                proposal_digest: digest,
                valid: false,
                errors: failure.errors.into_iter().map(structural_denial).collect(),
                waves: Vec::new(),
                automatic_eligible: false,
            });
        }
    };

    let resolved = match validate_proposal_against_host_state(&canonical, manifest, registry) {
        Ok(resolved) => resolved,
        Err(failure) => {
            return Ok(ParallelScheduleResponse {
                proposal_id,
                proposal_digest: digest,
                valid: false,
                errors: failure.errors.into_iter().map(cross_denial).collect(),
                waves: Vec::new(),
                automatic_eligible: false,
            });
        }
    };

    let schedule = match compute_parallel_schedule(&canonical, schedule_limits) {
        Ok(schedule) => schedule,
        Err(failure) => {
            return Ok(ParallelScheduleResponse {
                proposal_id,
                proposal_digest: digest,
                valid: false,
                errors: failure.errors.into_iter().map(schedule_denial).collect(),
                waves: Vec::new(),
                automatic_eligible: false,
            });
        }
    };

    if let Err(failure) = enforce_pure_read_only_parallelism(&schedule, &resolved) {
        return Ok(ParallelScheduleResponse {
            proposal_id,
            proposal_digest: digest,
            valid: false,
            errors: failure
                .errors
                .into_iter()
                .map(authorization_denial)
                .collect(),
            waves: Vec::new(),
            automatic_eligible: false,
        });
    }

    Ok(ParallelScheduleResponse {
        proposal_id,
        proposal_digest: digest,
        valid: true,
        errors: Vec::new(),
        waves: schedule.waves,
        automatic_eligible: traverse_runtime::proposal::proposal_is_automatic_eligible(&resolved),
    })
}

/// Everything [`execute_parallel_proposal_via_mcp`] needs beyond the
/// caller's runtime and shared authorization/quota state.
pub struct ParallelProposalExecutionRequest<'a> {
    pub proposal_json: &'a str,
    pub manifest: &'a ApplicationBundleManifest,
    pub registry: &'a CapabilityRegistry,
    pub proposal_limits: &'a ProposalLimits,
    pub schedule_limits: &'a ParallelScheduleLimits,
    pub execution_limits: &'a ParallelExecutionLimits,
    pub snapshots: &'a SnapshotDigests,
    pub approval_token: Option<&'a str>,
    pub expected_token_issuer: &'a str,
    pub expected_token_audience: &'a str,
    pub token_verifying_keys_by_key_id: &'a HashMap<String, VerifyingKey>,
    pub principal: &'a str,
    pub app_id: &'a str,
}

/// Authorizes and executes a proposal's bounded parallel schedule end to end
/// (spec 110 FR-001 through FR-005, reusing spec 109 FR-006 through FR-009
/// for authorization, quotas, and tracing). A denial for any reason —
/// invalid proposal, schedule bound exceeded, a non-`pure_read` node in a
/// concurrent wave, missing/invalid approval token, exhausted quota — is a
/// normal structured response, never an [`McpError`].
///
/// # Errors
///
/// Returns [`McpError`] only when `proposal_json` is not valid JSON.
pub fn execute_parallel_proposal_via_mcp<E: LocalExecutor>(
    runtime: &Runtime<E>,
    request: &ParallelProposalExecutionRequest<'_>,
    token_store: &ApprovalTokenStore,
    quota_tracker: &QuotaTracker,
    quota_limits: &QuotaLimits,
) -> Result<ProposalExecutionResponse, McpError>
where
    Runtime<E>: Sync,
{
    let (proposal, _proposal_id, digest) = parse_and_digest(request.proposal_json)?;
    let workspace_id = proposal.workspace_id.clone();
    let snapshot_digest = proposal_snapshot_digest(&digest, request.snapshots);

    let canonical = match canonicalize_proposal(proposal, request.proposal_limits) {
        Ok(canonical) => canonical,
        Err(failure) => {
            return Ok(ProposalExecutionResponse::Denied {
                code: "invalid_proposal".to_string(),
                message: format!("{} structural validation error(s)", failure.errors.len()),
            });
        }
    };

    let resolved = match validate_proposal_against_host_state(
        &canonical,
        request.manifest,
        request.registry,
    ) {
        Ok(resolved) => resolved,
        Err(failure) => {
            return Ok(ProposalExecutionResponse::Denied {
                code: "invalid_proposal".to_string(),
                message: format!("{} cross-validation error(s)", failure.errors.len()),
            });
        }
    };

    let schedule = match compute_parallel_schedule(&canonical, request.schedule_limits) {
        Ok(schedule) => schedule,
        Err(failure) => {
            return Ok(ProposalExecutionResponse::Denied {
                code: "invalid_parallel_schedule".to_string(),
                message: format!("{} schedule bound violation(s)", failure.errors.len()),
            });
        }
    };

    if let Err(failure) = enforce_pure_read_only_parallelism(&schedule, &resolved) {
        return Ok(ProposalExecutionResponse::Denied {
            code: "concurrent_side_effect_denied".to_string(),
            message: format!(
                "{} node(s) denied concurrent execution",
                failure.errors.len()
            ),
        });
    }

    let authorized =
        crate::tools::proposals::authorize_and_reserve_quota(&AuthorizeAndReserveQuotaRequest {
            resolved: &resolved,
            digest: &digest,
            snapshot_digest: &snapshot_digest,
            workspace_id: &workspace_id,
            approval_token: request.approval_token,
            expected_token_issuer: request.expected_token_issuer,
            expected_token_audience: request.expected_token_audience,
            token_verifying_keys_by_key_id: request.token_verifying_keys_by_key_id,
            token_store,
            quota_tracker,
            quota_limits,
            principal: request.principal,
            app_id: request.app_id,
        });
    let authorized = match authorized {
        Ok(authorized) => authorized,
        Err(denial) => return Ok(*denial),
    };

    let trace = execute_parallel_proposal(
        runtime,
        &canonical,
        &schedule,
        authorized.summary,
        &digest,
        &snapshot_digest,
        request.execution_limits,
    );
    drop(authorized.reservation);
    Ok(ProposalExecutionResponse::Trace(trace))
}

fn schedule_denial(error: traverse_contracts::ParallelScheduleError) -> ProposalDenial {
    ProposalDenial {
        code: debug_enum_to_snake_case(&format!("{:?}", error.code)),
        path: error.path,
        message: error.message,
    }
}

fn authorization_denial(
    error: traverse_runtime::parallel_proposal::ParallelAuthorizationError,
) -> ProposalDenial {
    ProposalDenial {
        code: debug_enum_to_snake_case(&format!("{:?}", error.code)),
        path: error.path,
        message: error.message,
    }
}

/// Converts a Rust `Debug`-formatted `PascalCase` enum variant into the
/// stable `snake_case` string used for every machine-readable code this
/// module emits (spec 109 FR-010, carried over for spec 110).
fn debug_enum_to_snake_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}
