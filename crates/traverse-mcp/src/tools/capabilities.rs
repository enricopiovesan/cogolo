//! MCP tool surfaces for capability discovery.
//!
//! Governed by spec 015-capability-discovery-mcp

use serde::{Deserialize, Serialize};
use traverse_contracts::{ExecutionTarget, ServiceType};
use traverse_registry::{CapabilityRegistry, DiscoveryQuery, LookupScope};

use crate::{McpError, McpErrorCode};

/// Optional filter for [`list_capabilities`].
#[derive(Debug, Clone, Default)]
pub struct CapabilityFilter {
    /// When set, only return capabilities with this service type.
    pub service_type: Option<ServiceType>,
    /// When non-empty, only return capabilities whose `permitted_targets`
    /// include all of the listed targets.
    pub permitted_targets: Vec<ExecutionTarget>,
}

/// Summary record returned by [`list_capabilities`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySummary {
    /// Capability identifier.
    pub id: String,
    /// Capability display name.
    pub name: String,
    /// Service type classification.
    pub service_type: ServiceType,
    /// Execution targets this capability may run on.
    pub permitted_targets: Vec<ExecutionTarget>,
    /// Short human-readable description.
    pub description: String,
}

/// List all capabilities, optionally filtered by `service_type` or `permitted_targets`.
///
/// Uses `LookupScope::PreferPrivate` internally so private overrides are preferred.
#[must_use]
pub fn list_capabilities(
    registry: &CapabilityRegistry,
    filter: Option<&CapabilityFilter>,
) -> Vec<CapabilitySummary> {
    let entries = registry.discover(LookupScope::PreferPrivate, &DiscoveryQuery::default());

    entries
        .into_iter()
        .filter_map(|entry| {
            registry.find_exact(LookupScope::PreferPrivate, &entry.id, &entry.version)
        })
        .filter(|cap| {
            let Some(f) = filter else { return true };

            let service_type_ok = f
                .service_type
                .as_ref()
                .is_none_or(|st| &cap.contract.service_type == st);

            let targets_ok = f.permitted_targets.is_empty()
                || f.permitted_targets
                    .iter()
                    .all(|t| cap.contract.permitted_targets.contains(t));

            service_type_ok && targets_ok
        })
        .map(|cap| CapabilitySummary {
            id: cap.contract.id.clone(),
            name: cap.contract.name.clone(),
            service_type: cap.contract.service_type.clone(),
            permitted_targets: cap.contract.permitted_targets.clone(),
            description: cap.contract.description.clone(),
        })
        .collect()
}

/// Return the full contract JSON for a capability identified by `capability_id`.
///
/// Finds the latest registered version for the given id. Uses
/// `LookupScope::PreferPrivate` so private overrides are preferred.
///
/// # Errors
///
/// Returns [`McpError`] with code `NotFound` when no matching capability exists in the registry.
pub fn get_capability(
    registry: &CapabilityRegistry,
    capability_id: &str,
) -> Result<serde_json::Value, McpError> {
    let entries = registry.discover(LookupScope::PreferPrivate, &DiscoveryQuery::default());

    let entry = entries
        .into_iter()
        .find(|e| e.id == capability_id)
        .ok_or_else(|| McpError {
            code: McpErrorCode::NotFound,
            message: format!("capability '{capability_id}' not found"),
        })?;

    let resolved = registry
        .find_exact(LookupScope::PreferPrivate, &entry.id, &entry.version)
        .ok_or_else(|| McpError {
            code: McpErrorCode::NotFound,
            message: format!("capability '{capability_id}' not found"),
        })?;

    serde_json::to_value(&resolved.contract).map_err(|e| McpError {
        code: McpErrorCode::InvalidRequest,
        message: e.to_string(),
    })
}
