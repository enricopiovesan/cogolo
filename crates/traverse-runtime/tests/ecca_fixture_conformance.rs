//! Portable ECCA event-product descriptor fixture conformance (registry 0.11.0 corpus).

use std::path::PathBuf;

use traverse_runtime::events::run_descriptor_fixture_conformance;

#[test]
fn portable_registry_descriptor_fixtures_conform() -> Result<(), String> {
    let fixtures_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ecca-event-products");
    let report = run_descriptor_fixture_conformance(&fixtures_dir)?;
    if !report.failures.is_empty() {
        return Err(format!("fixture conformance failures: {:?}", report.failures));
    }
    if report.passed != 17 {
        return Err(format!("expected 17 fixtures to pass, got {}", report.passed));
    }
    Ok(())
}
