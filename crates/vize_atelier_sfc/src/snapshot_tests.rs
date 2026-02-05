//! Snapshot tests for TypeScript output mode
//!
//! This module tests the TypeScript output generation for Vue SFCs.
//! It uses the insta crate for snapshot testing, with snapshots stored
//! in tests/snapshots/sfc/ts/.
//!
//! The test cases are loaded from TOML fixtures in tests/fixtures/sfc/.

use crate::{compile_sfc, parse_sfc, SfcCompileOptions};
use serde::Deserialize;
use std::path::PathBuf;

/// A test case from a TOML fixture
#[derive(Debug, Deserialize)]
struct TestCase {
    name: String,
    input: String,
    #[allow(dead_code)]
    expected: Option<String>,
}

/// A fixture file containing multiple test cases
#[derive(Debug, Deserialize)]
struct Fixture {
    #[allow(dead_code)]
    mode: Option<String>,
    cases: Vec<TestCase>,
}

/// Get the path to the tests/fixtures directory
fn fixtures_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

/// Get the path to the tests/snapshots directory
fn snapshots_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("snapshots")
}

/// Load a fixture from a TOML file
fn load_fixture(path: &PathBuf) -> Result<Fixture, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let fixture: Fixture = toml::from_str(&content)?;
    Ok(fixture)
}

/// Normalize a test case name to a valid snapshot file name
fn normalize_name(name: &str) -> String {
    name.to_lowercase()
        .replace([' ', '-'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Compile an SFC to TypeScript output
fn compile_sfc_ts(input: &str) -> String {
    let descriptor = match parse_sfc(input, Default::default()) {
        Ok(d) => d,
        Err(e) => return format!("Parse error: {:?}", e),
    };

    let mut options = SfcCompileOptions::default();
    // Enable TypeScript output mode
    options.script.is_ts = true;
    options.template.is_ts = true;
    options.script.id = Some("test.vue".to_string());

    match compile_sfc(&descriptor, options) {
        Ok(result) => result.code,
        Err(e) => format!("Compile error: {:?}", e),
    }
}

#[test]
fn test_script_setup_ts_snapshots() {
    let snapshot_path = snapshots_path().join("sfc").join("ts");
    std::fs::create_dir_all(&snapshot_path).ok();

    let fixture_path = fixtures_path().join("sfc").join("script-setup.toml");
    let fixture = load_fixture(&fixture_path).expect("Failed to load fixture");

    for case in &fixture.cases {
        let normalized_name = normalize_name(&case.name);
        let ts_output = compile_sfc_ts(&case.input);

        insta::with_settings!({
            snapshot_path => &snapshot_path,
            prepend_module_to_snapshot => false,
            snapshot_suffix => "",
        }, {
            insta::assert_snapshot!(format!("script_setup__{}", normalized_name), ts_output);
        });
    }
}

#[test]
fn test_basic_sfc_ts_snapshots() {
    let snapshot_path = snapshots_path().join("sfc").join("ts");
    std::fs::create_dir_all(&snapshot_path).ok();

    let fixture_path = fixtures_path().join("sfc").join("basic.toml");
    let fixture = load_fixture(&fixture_path).expect("Failed to load fixture");

    for case in &fixture.cases {
        let normalized_name = normalize_name(&case.name);
        let ts_output = compile_sfc_ts(&case.input);

        insta::with_settings!({
            snapshot_path => &snapshot_path,
            prepend_module_to_snapshot => false,
            snapshot_suffix => "",
        }, {
            insta::assert_snapshot!(format!("basic__{}", normalized_name), ts_output);
        });
    }
}

#[test]
fn test_patches_sfc_ts_snapshots() {
    let snapshot_path = snapshots_path().join("sfc").join("ts");
    std::fs::create_dir_all(&snapshot_path).ok();

    let fixture_path = fixtures_path().join("sfc").join("patches.toml");
    let fixture = load_fixture(&fixture_path).expect("Failed to load fixture");

    for case in &fixture.cases {
        let normalized_name = normalize_name(&case.name);
        let ts_output = compile_sfc_ts(&case.input);

        insta::with_settings!({
            snapshot_path => &snapshot_path,
            prepend_module_to_snapshot => false,
            snapshot_suffix => "",
        }, {
            insta::assert_snapshot!(format!("patches__{}", normalized_name), ts_output);
        });
    }
}
