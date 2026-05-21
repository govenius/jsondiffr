//! Integration test that runs every fixture under `../reference_data/`
//! through the patcher and compares the result with the stored target.

use std::fs;
use std::path::PathBuf;

use jsondiffr::{parse_diff, parse_source, patch};

fn reference_data_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .expect("crate root must have a parent directory")
        .join("reference_data")
}

#[test]
fn all_reference_fixtures_pass() {
    let dir = reference_data_dir();
    let manifest_path = dir.join("_manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("parse _manifest.json");
    let cases: Vec<String> = manifest["cases"]
        .as_array()
        .expect("_manifest.json cases must be array")
        .iter()
        .map(|v| v.as_str().expect("case id must be a string").to_owned())
        .collect();

    assert!(
        !cases.is_empty(),
        "_manifest.json declared no cases — nothing to test"
    );

    let mut failures: Vec<String> = Vec::new();

    for case_id in &cases {
        let path = dir.join(format!("{case_id}.json"));
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let fixture: serde_json::Value =
            serde_json::from_str(&raw).expect("fixture must be valid JSON");

        let source = parse_source(fixture["source"].clone());
        let target = parse_source(fixture["target"].clone());
        let parsed_diff = match parse_diff(fixture["diff"].clone()) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!("{case_id}: parse_diff failed: {e}"));
                continue;
            }
        };

        match patch(&source, &parsed_diff) {
            Ok(actual) if actual == target => {}
            Ok(actual) => failures.push(format!(
                "{case_id}: patched value != target\n  got:  {actual:#?}\n  want: {target:#?}"
            )),
            Err(e) => failures.push(format!("{case_id}: patch returned error: {e}")),
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}/{} fixture(s) failed:\n\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n\n")
        );
    }

    println!("{} reference fixtures passed", cases.len());
}
