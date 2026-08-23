//! MCP tool surfaces for the runtime workflow proposal lifecycle.
//!
//! Governed by spec `109-runtime-workflow-proposals`. Mirrors the
//! `tools::capabilities` pattern: plain, fully-tested Rust functions form the
//! public MCP surface (spec 015's precedent), independent of the separate
//! `stdio_server.rs` reference host transport.

use ed25519_dalek::VerifyingKey;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use traverse_contracts::{
    ProposalLimits, ProposalValidationError as StructuralError, SnapshotDigests, WorkflowProposal,
    canonicalize_proposal, proposal_digest, proposal_snapshot_digest,
};
use traverse_registry::{ApplicationBundleManifest, CapabilityRegistry};
use traverse_runtime::proposal::{
    ApprovalTokenStore, ApprovalTokenVerificationContext, AuthorizationDecision,
    AuthorizationSummary, ProposalCrossValidationError as CrossError, ProposalTrace, QuotaLimits,
    QuotaTracker, execute_proposal, proposal_is_automatic_eligible,
    validate_proposal_against_host_state, verify_approval_token,
};
use traverse_runtime::{LocalExecutor, Runtime};

use crate::{McpError, McpErrorCode};

/// One stable, machine-readable, secret-free denial (spec 109 FR-010).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProposalDenial {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalValidationResponse {
    pub proposal_id: String,
    pub proposal_digest: String,
    pub valid: bool,
    pub errors: Vec<ProposalDenial>,
}

/// Parses, canonicalizes, and cross-validates a proposal against the loaded
/// manifest and registry (spec 109 FR-001 validation feedback / compatibility
/// inspection). A structurally or semantically invalid proposal is a normal,
/// structured response, never an [`McpError`] — matching FR-010's stable
/// denial-code contract.
///
/// # Errors
///
/// Returns [`McpError`] only when `proposal_json` is not valid JSON or does
/// not deserialize into the proposal wire shape at all.
pub fn validate_proposal(
    proposal_json: &str,
    manifest: &ApplicationBundleManifest,
    registry: &CapabilityRegistry,
    limits: &ProposalLimits,
) -> Result<ProposalValidationResponse, McpError> {
    let (proposal, proposal_id, digest) = parse_and_digest(proposal_json)?;

    let canonical = match canonicalize_proposal(proposal, limits) {
        Ok(canonical) => canonical,
        Err(failure) => {
            return Ok(ProposalValidationResponse {
                proposal_id,
                proposal_digest: digest,
                valid: false,
                errors: failure.errors.into_iter().map(structural_denial).collect(),
            });
        }
    };

    match validate_proposal_against_host_state(&canonical, manifest, registry) {
        Ok(_resolved) => Ok(ProposalValidationResponse {
            proposal_id,
            proposal_digest: digest,
            valid: true,
            errors: Vec::new(),
        }),
        Err(failure) => Ok(ProposalValidationResponse {
            proposal_id,
            proposal_digest: digest,
            valid: false,
            errors: failure.errors.into_iter().map(cross_denial).collect(),
        }),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalSubmissionResponse {
    pub proposal_id: String,
    pub proposal_digest: String,
    pub snapshot_digest: String,
    pub valid: bool,
    pub errors: Vec<ProposalDenial>,
    pub automatic_eligible: bool,
}

/// Submits a proposal: validates it, then binds its digest to the pinned
/// governing snapshots (spec 109 FR-003) and decides automatic eligibility
/// (FR-006). P1 has no server-side proposal catalog — submission does not
/// persist anything; the caller re-presents the same JSON to `execute`.
///
/// # Errors
///
/// Returns [`McpError`] only when `proposal_json` is not valid JSON.
pub fn submit_proposal(
    proposal_json: &str,
    manifest: &ApplicationBundleManifest,
    registry: &CapabilityRegistry,
    limits: &ProposalLimits,
    snapshots: &SnapshotDigests,
) -> Result<ProposalSubmissionResponse, McpError> {
    let (proposal, proposal_id, digest) = parse_and_digest(proposal_json)?;
    let snapshot_digest = proposal_snapshot_digest(&digest, snapshots);

    let canonical = match canonicalize_proposal(proposal, limits) {
        Ok(canonical) => canonical,
        Err(failure) => {
            return Ok(ProposalSubmissionResponse {
                proposal_id,
                proposal_digest: digest,
                snapshot_digest,
                valid: false,
                errors: failure.errors.into_iter().map(structural_denial).collect(),
                automatic_eligible: false,
            });
        }
    };

    match validate_proposal_against_host_state(&canonical, manifest, registry) {
        Ok(resolved) => Ok(ProposalSubmissionResponse {
            proposal_id,
            proposal_digest: digest,
            snapshot_digest,
            valid: true,
            errors: Vec::new(),
            automatic_eligible: proposal_is_automatic_eligible(&resolved),
        }),
        Err(failure) => Ok(ProposalSubmissionResponse {
            proposal_id,
            proposal_digest: digest,
            snapshot_digest,
            valid: false,
            errors: failure.errors.into_iter().map(cross_denial).collect(),
            automatic_eligible: false,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AuthorizationState {
    Invalid { errors: Vec<ProposalDenial> },
    Automatic,
    RequiresApprovalToken,
}

/// Reports whether a proposal is automatic-eligible or requires a verified
/// approval token (spec 109 FR-006), without executing anything.
///
/// # Errors
///
/// Returns [`McpError`] only when `proposal_json` is not valid JSON.
pub fn authorization_state(
    proposal_json: &str,
    manifest: &ApplicationBundleManifest,
    registry: &CapabilityRegistry,
    limits: &ProposalLimits,
) -> Result<AuthorizationState, McpError> {
    let (proposal, _proposal_id, _digest) = parse_and_digest(proposal_json)?;

    let canonical = match canonicalize_proposal(proposal, limits) {
        Ok(canonical) => canonical,
        Err(failure) => {
            return Ok(AuthorizationState::Invalid {
                errors: failure.errors.into_iter().map(structural_denial).collect(),
            });
        }
    };

    match validate_proposal_against_host_state(&canonical, manifest, registry) {
        Ok(resolved) => Ok(if proposal_is_automatic_eligible(&resolved) {
            AuthorizationState::Automatic
        } else {
            AuthorizationState::RequiresApprovalToken
        }),
        Err(failure) => Ok(AuthorizationState::Invalid {
            errors: failure.errors.into_iter().map(cross_denial).collect(),
        }),
    }
}

/// Everything [`execute_proposal_via_mcp`] needs beyond the caller's runtime
/// and shared authorization/quota state — bundled to keep the function
/// signature reasonable given spec 109's many independently-required checks.
pub struct ProposalExecutionRequest<'a> {
    pub proposal_json: &'a str,
    pub manifest: &'a ApplicationBundleManifest,
    pub registry: &'a CapabilityRegistry,
    pub limits: &'a ProposalLimits,
    pub snapshots: &'a SnapshotDigests,
    pub approval_token: Option<&'a str>,
    pub expected_token_issuer: &'a str,
    pub expected_token_audience: &'a str,
    pub token_verifying_keys_by_key_id: &'a HashMap<String, VerifyingKey>,
    pub principal: &'a str,
    pub app_id: &'a str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposalExecutionResponse {
    Trace(ProposalTrace),
    Denied { code: String, message: String },
}

/// Authorizes and executes a proposal end to end (spec 109 FR-006 through
/// FR-009): validates it, decides or verifies authorization, reserves a
/// per-principal/app/workspace quota slot (FR-007b), and runs the bounded
/// sequential DAG (FR-007, FR-008). A denial for any reason — invalid
/// proposal, missing/invalid approval token, exhausted quota — is a normal
/// structured response, never an [`McpError`].
///
/// # Errors
///
/// Returns [`McpError`] only when `proposal_json` is not valid JSON.
pub fn execute_proposal_via_mcp<E: LocalExecutor>(
    runtime: &Runtime<E>,
    request: &ProposalExecutionRequest<'_>,
    token_store: &ApprovalTokenStore,
    quota_tracker: &QuotaTracker,
    quota_limits: &QuotaLimits,
) -> Result<ProposalExecutionResponse, McpError> {
    let (proposal, _proposal_id, digest) = parse_and_digest(request.proposal_json)?;
    let workspace_id = proposal.workspace_id.clone();
    let snapshot_digest = proposal_snapshot_digest(&digest, request.snapshots);

    let canonical = match canonicalize_proposal(proposal, request.limits) {
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

    let authorization = if proposal_is_automatic_eligible(&resolved) {
        AuthorizationDecision::Automatic
    } else {
        let Some(token) = request.approval_token else {
            return Ok(ProposalExecutionResponse::Denied {
                code: "approval_token_required".to_string(),
                message: "this proposal requires a verified approval token".to_string(),
            });
        };
        let verification_context = ApprovalTokenVerificationContext {
            expected_issuer: request.expected_token_issuer,
            expected_audience: request.expected_token_audience,
            expected_workspace_id: &workspace_id,
            expected_proposal_digest: &digest,
            expected_snapshot_digest: &snapshot_digest,
            verifying_keys_by_key_id: request.token_verifying_keys_by_key_id,
        };
        let claims = match verify_approval_token(token, &verification_context) {
            Ok(claims) => claims,
            Err(error) => {
                return Ok(ProposalExecutionResponse::Denied {
                    code: token_error_code(&error.code),
                    message: error.message,
                });
            }
        };
        if let Err(error) = token_store.check_and_record_use(&claims) {
            return Ok(ProposalExecutionResponse::Denied {
                code: token_error_code(&error.code),
                message: error.message,
            });
        }
        AuthorizationDecision::Approved(Box::new(claims))
    };

    let reservation = match quota_tracker.reserve(
        request.principal,
        request.app_id,
        &workspace_id,
        quota_limits,
    ) {
        Ok(reservation) => reservation,
        Err(denial) => {
            return Ok(ProposalExecutionResponse::Denied {
                code: format!("quota_exhausted_{}", denial.scope),
                message: denial.message,
            });
        }
    };

    let authorization_summary = match &authorization {
        AuthorizationDecision::Automatic => AuthorizationSummary {
            automatic: true,
            approval_token_id: None,
        },
        AuthorizationDecision::Approved(claims) => AuthorizationSummary {
            automatic: false,
            approval_token_id: Some(claims.token_id.clone()),
        },
    };

    let trace = execute_proposal(
        runtime,
        &canonical,
        &resolved,
        authorization_summary,
        &digest,
        &snapshot_digest,
    );
    drop(reservation);
    Ok(ProposalExecutionResponse::Trace(trace))
}

/// Renders a completed execution's redacted trace for MCP observation (spec
/// 109 FR-001 observation, FR-009). The trace itself already excludes raw
/// payloads and secrets — this is a plain JSON projection, not a second
/// redaction pass.
#[must_use]
pub fn observe_proposal(trace: &ProposalTrace) -> Value {
    serde_json::to_value(trace).unwrap_or(Value::Null)
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposalExportResponse {
    pub proposal: WorkflowProposal,
    pub proposal_digest: String,
    pub execution_order: Vec<String>,
}

/// Exports a proposal's canonical form and digest so an external party can
/// independently re-derive the identical digest (spec 109 FR-001 export,
/// FR-007a: "re-submitting pinned identical inputs produces the same
/// proposal digest"). Performs structural validation only — no manifest or
/// registry cross-check.
///
/// # Errors
///
/// Returns [`McpError`] when `proposal_json` is not valid JSON or is not
/// structurally valid.
pub fn export_proposal(
    proposal_json: &str,
    limits: &ProposalLimits,
) -> Result<ProposalExportResponse, McpError> {
    let (proposal, _proposal_id, digest) = parse_and_digest(proposal_json)?;
    let canonical = canonicalize_proposal(proposal, limits).map_err(|failure| McpError {
        code: McpErrorCode::ValidationFailed,
        message: format!(
            "proposal is not structurally valid ({} error(s))",
            failure.errors.len()
        ),
    })?;
    Ok(ProposalExportResponse {
        proposal: canonical.proposal,
        proposal_digest: digest,
        execution_order: canonical.execution_order,
    })
}

fn parse_and_digest(proposal_json: &str) -> Result<(WorkflowProposal, String, String), McpError> {
    let proposal: WorkflowProposal = serde_json::from_str(proposal_json).map_err(|e| McpError {
        code: McpErrorCode::InvalidRequest,
        message: format!("proposal is not valid JSON: {e}"),
    })?;
    let digest = proposal_digest(&proposal);
    let proposal_id = proposal.proposal_id.clone();
    Ok((proposal, proposal_id, digest))
}

fn structural_denial(error: StructuralError) -> ProposalDenial {
    ProposalDenial {
        code: debug_enum_to_snake_case(&format!("{:?}", error.code)),
        path: error.path,
        message: error.message,
    }
}

fn cross_denial(error: CrossError) -> ProposalDenial {
    ProposalDenial {
        code: debug_enum_to_snake_case(&format!("{:?}", error.code)),
        path: error.path,
        message: error.message,
    }
}

fn token_error_code(code: &traverse_runtime::proposal::ApprovalTokenErrorCode) -> String {
    debug_enum_to_snake_case(&format!("{code:?}"))
}

/// Converts a Rust `Debug`-formatted `PascalCase` enum variant into the
/// stable `snake_case` string used for every machine-readable code this
/// module emits (spec 109 FR-010).
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
