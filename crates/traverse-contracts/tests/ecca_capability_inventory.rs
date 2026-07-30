//! Regression test for the ECCA existing-catalog migration (spec 534-ecca-event-products, FR-020).
//!
//! Governed by traverse-framework/traverse#899. Fails the build when a published
//! capability contract has no inventory classification, or when a capability that
//! declares `event_emission`/`emits` does not have a matching, validator-parseable
//! event contract that lists it as a publisher (and, where a `consumes` reference
//! exists, an upstream event contract that lists it as a subscriber).

use std::{collections::BTreeMap, fs, path::Path, path::PathBuf};

use serde_json::Value;
use traverse_contracts::{SideEffectKind, parse_contract, parse_event_contract};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Recursively collect every `contract.json` file under `dir`.
fn find_contract_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| format!("{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            find_contract_files(&path, out)?;
        } else if path.file_name().and_then(|name| name.to_str()) == Some("contract.json") {
            out.push(path);
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn find_event_contract_path(root: &Path, event_id: &str) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    find_contract_files(&root.join("contracts/examples"), &mut candidates)?;
    candidates
        .into_iter()
        .find(|path| {
            read_json(path)
                .ok()
                .and_then(|value| {
                    let is_event = value.get("kind")?.as_str()? == "event_contract";
                    let id = value.get("id")?.as_str()?;
                    Some(is_event && id == event_id)
                })
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("no event contract file found for event id '{event_id}'"))
}

fn assert_declared_publisher(
    root: &Path,
    capability_id: &str,
    event_id: &str,
) -> Result<(), String> {
    let event_path = find_event_contract_path(root, event_id)?;
    let event_json = fs::read_to_string(&event_path)
        .map_err(|error| format!("{}: {error}", event_path.display()))?;
    let event_contract = parse_event_contract(&event_json)
        .map_err(|error| format!("{}: {error:?}", event_path.display()))?;

    if !event_contract
        .publishers
        .iter()
        .any(|publisher| publisher.capability_id == capability_id)
    {
        return Err(format!(
            "'{capability_id}' emits '{event_id}' but is not declared as a publisher on that \
             event contract at {}",
            event_path.display()
        ));
    }
    Ok(())
}

fn assert_declared_subscriber(
    root: &Path,
    capability_id: &str,
    event_id: &str,
) -> Result<(), String> {
    let event_path = find_event_contract_path(root, event_id)?;
    let event_json = fs::read_to_string(&event_path)
        .map_err(|error| format!("{}: {error}", event_path.display()))?;
    let event_contract = parse_event_contract(&event_json)
        .map_err(|error| format!("{}: {error:?}", event_path.display()))?;

    if !event_contract
        .subscribers
        .iter()
        .any(|subscriber| subscriber.capability_id == capability_id)
    {
        return Err(format!(
            "'{capability_id}' consumes '{event_id}' but is not declared as a subscriber on \
             that event contract at {}",
            event_path.display()
        ));
    }
    Ok(())
}

/// Checks one published capability against its manifest entry. Returns `Err` describing the
/// first FR-020 violation found.
fn check_capability(root: &Path, id: &str, path: &Path, entry: &Value) -> Result<(), String> {
    let classification = entry
        .get("classification")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("manifest entry for '{id}' missing classification"))?;

    let contract_json =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let contract =
        parse_contract(&contract_json).map_err(|error| format!("{}: {error:?}", path.display()))?;

    let declares_event_emission = contract
        .side_effects
        .iter()
        .any(|effect| matches!(effect.kind, SideEffectKind::EventEmission));
    let has_emits = !contract.emits.is_empty();

    if !declares_event_emission && !has_emits {
        if classification != "no-event-required" {
            return Err(format!(
                "'{id}' has no event_emission side effect and no emits, so it must be \
                 classified 'no-event-required', not '{classification}'"
            ));
        }
        let evidence = entry.get("evidence").and_then(Value::as_str).unwrap_or("");
        if evidence.trim().is_empty() {
            return Err(format!(
                "'{id}' is classified no-event-required but has no documented evidence"
            ));
        }
        return Ok(());
    }

    if classification != "governed-event-declared" {
        return Err(format!(
            "'{id}' declares event_emission/emits but is classified '{classification}'"
        ));
    }
    if !(declares_event_emission && has_emits) {
        return Err(format!(
            "'{id}' must declare both an event_emission side effect and a non-empty emits \
             list to be classified governed-event-declared"
        ));
    }

    for event_ref in &contract.emits {
        assert_declared_publisher(root, id, &event_ref.event_id)?;
    }
    for consumed in &contract.consumes {
        assert_declared_subscriber(root, id, &consumed.event_id)?;
    }
    Ok(())
}

#[test]
fn every_published_capability_has_an_inventory_classification() -> Result<(), String> {
    let root = repo_root();

    let mut contract_files = Vec::new();
    find_contract_files(&root.join("contracts/examples"), &mut contract_files)?;
    find_contract_files(&root.join("contracts/inference"), &mut contract_files)?;

    let published_capabilities: BTreeMap<String, PathBuf> = contract_files
        .into_iter()
        .filter_map(|path| {
            let value = read_json(&path).ok()?;
            if value.get("kind")?.as_str()? != "capability_contract" {
                return None;
            }
            let id = value.get("id")?.as_str()?.to_string();
            Some((id, path))
        })
        .collect();

    let manifest = read_json(&root.join("contracts/governance/ecca-capability-inventory.json"))?;
    let manifest_entries = manifest
        .get("capabilities")
        .and_then(Value::as_array)
        .ok_or("manifest missing capabilities array")?;

    let mut manifest_ids: BTreeMap<String, &Value> = BTreeMap::new();
    for entry in manifest_entries {
        let id = entry
            .get("capability_id")
            .and_then(Value::as_str)
            .ok_or("manifest entry missing capability_id")?
            .to_string();
        manifest_ids.insert(id, entry);
    }

    for id in published_capabilities.keys() {
        if !manifest_ids.contains_key(id) {
            return Err(format!(
                "published capability '{id}' has no ECCA inventory classification in \
                 contracts/governance/ecca-capability-inventory.json; FR-020 requires a \
                 validator-backed classification before its next publication"
            ));
        }
    }
    for id in manifest_ids.keys() {
        if !published_capabilities.contains_key(id) {
            return Err(format!(
                "ECCA inventory manifest references '{id}' but no such published capability \
                 contract exists on disk; remove the stale entry"
            ));
        }
    }

    for (id, path) in &published_capabilities {
        let entry = manifest_ids[id];
        check_capability(&root, id, path, entry)?;
    }

    Ok(())
}
