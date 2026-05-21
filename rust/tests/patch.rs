//! Unit tests for the patcher (FORMAT.md §2 — apply rules).
//!
//! Mirrors the §2 patch-behavior group from `TestFormatConformance`
//! in `tests/test_jsondiff.py`.

use jsondiffr::{parse_diff, parse_source, patch, PatchError};
use serde_json::json;

#[test]
fn empty_diff_is_identity() {
    let empty_diff = parse_diff(json!({})).unwrap();
    let samples = [
        json!(null),
        json!(true),
        json!(0),
        json!("x"),
        json!({}),
        json!([]),
        json!({"a": 1}),
        json!([1, 2, 3]),
    ];
    for v in &samples {
        let s = parse_source(v.clone());
        let got = patch(&s, &empty_diff).unwrap();
        assert_eq!(
            got,
            parse_source(v.clone()),
            "empty diff should be identity for {v}"
        );
    }
}

#[test]
fn non_object_diff_is_replacement() {
    // Array diff → wholesale replace.
    let source = parse_source(json!({"a": 1}));
    let diff = parse_diff(json!([1, 2])).unwrap();
    assert_eq!(patch(&source, &diff).unwrap(), parse_source(json!([1, 2])));

    // Scalar diff → wholesale replace.
    let source = parse_source(json!({"a": 1}));
    let diff = parse_diff(json!(42)).unwrap();
    assert_eq!(patch(&source, &diff).unwrap(), parse_source(json!(42)));

    // Null diff → wholesale replace.
    let source = parse_source(json!([1, 2, 3]));
    let diff = parse_diff(json!(null)).unwrap();
    assert_eq!(patch(&source, &diff).unwrap(), parse_source(json!(null)));
}

#[test]
fn replace_sentinel_short_circuits() {
    let source = parse_source(json!({"a": 1}));
    let diff = parse_diff(json!({"$replace": {"x": 9}})).unwrap();
    assert_eq!(patch(&source, &diff).unwrap(), parse_source(json!({"x": 9})));

    let source = parse_source(json!([1, 2]));
    let diff = parse_diff(json!({"$replace": "gone"})).unwrap();
    assert_eq!(patch(&source, &diff).unwrap(), parse_source(json!("gone")));
}

#[test]
fn dict_delete_and_addition() {
    let source = parse_source(json!({"a": 1, "b": 2}));
    let diff = parse_diff(json!({"c": 3, "$delete": ["a"]})).unwrap();
    let want = parse_source(json!({"b": 2, "c": 3}));
    assert_eq!(patch(&source, &diff).unwrap(), want);
}

#[test]
fn list_delete_descending() {
    let source = parse_source(json!(["a", "b", "c", "d"]));
    let diff = parse_diff(json!({"$delete": [3, 1]})).unwrap();
    let want = parse_source(json!(["a", "c"]));
    assert_eq!(patch(&source, &diff).unwrap(), want);
}

#[test]
fn list_integer_keyed_subpatch() {
    let source = parse_source(json!([{"x": 1}, "b"]));
    let diff = parse_diff(json!({"0": {"x": 2}})).unwrap();
    let want = parse_source(json!([{"x": 2}, "b"]));
    assert_eq!(patch(&source, &diff).unwrap(), want);
}

#[test]
fn set_sentinel_returns_unsupported() {
    // The source type does not matter — the patcher rejects $add/$discard
    // before dispatching on source.
    let source = parse_source(json!([1, 2, 3]));
    let diff = parse_diff(json!({"$add": [9], "$discard": [2]})).unwrap();
    assert!(matches!(
        patch(&source, &diff),
        Err(PatchError::SetSourceUnsupported)
    ));
}

#[test]
fn basic_patch_assertions() {
    // Battery of (source, diff, target) triples covering shapes not already
    // exercised above. Mirrors a slice of Python's test_a.
    let cases = [
        // Scalar replacement.
        (json!(1), json!(2), json!(2)),
        // List with insert only.
        (
            json!(["a", "c"]),
            json!({"$insert": [[1, "b"]]}),
            json!(["a", "b", "c"]),
        ),
        // Dict sub-patch via an existing key.
        (
            json!({"k": {"a": 1, "b": 2}}),
            json!({"k": {"a": 9}}),
            json!({"k": {"a": 9, "b": 2}}),
        ),
        // List with delete + insert + integer-keyed sub-patch in one diff.
        (
            json!(["x", "a", {"v": 11}, "c", "x"]),
            json!({"1": {"v": 20}, "$insert": [[2, "b"]], "$delete": [4, 0]}),
            json!(["a", {"v": 20}, "b", "c"]),
        ),
    ];
    for (i, (src, dif, tgt)) in cases.iter().enumerate() {
        let s = parse_source(src.clone());
        let d = parse_diff(dif.clone()).unwrap();
        let t = parse_source(tgt.clone());
        assert_eq!(patch(&s, &d).unwrap(), t, "case {i} failed");
    }
}
