//! Placeholder for the differ test suite.
//!
//! The Rust differ is not yet implemented (see `src/lib.rs`). When it lands,
//! this file should grow the test functions described below. They mirror
//! the deferred portion of `tests/test_jsondiff.py` — Python tests whose
//! assertions go through `diff()` and therefore cannot be ported today.
//!
//! # Deferred until the Rust differ exists
//!
//! The following Python tests *cannot* be ported without `diff()`. Re-plan
//! when the differ lands:
//!
//! * All of `JsonDiffTests::test_a` (currently ~30 `diff(...)` equality cases).
//! * `test_marshal` — round-trip needs both `marshal()` and `unmarshal()`.
//! * `test_dump`, `test_compact_syntax` — hypothesis-driven property tests;
//!   would port to `proptest` once a differ is callable. Use the round-trip
//!   property `patch(a, diff(a, b)) == b`.
//! * The §3 *diff() behavior* group from `TestFormatConformance`:
//!   `diff_low_similarity_emits_bare_or_wrapped_b`,
//!   `diff_list_delete_positions_descending`,
//!   `diff_unchanged_subkeys_omitted`,
//!   `diff_against_empty_uses_replace_wrap`.
//! * The §4 invariants `diff_identity` and the cross-cutting
//!   `diff_does_not_mutate_inputs`.

#[cfg(test)]
mod tests {
    use jsondiffr::{diff, patch, parse_source};
    use serde_json::json;

    #[test]
    fn test_diff_identity() {
        let a = parse_source(json!({"a": 1, "b": [1, 2]}));
        let b = parse_source(json!({"a": 1, "b": [1, 2]}));
        
        // diff(a, a) should be Empty
        match diff(&a, &b) {
            jsondiffr::DiffValue::Empty => {}
            other => panic!("Expected DiffValue::Empty, got {:?}", other),
        }
    }

    #[test]
    fn test_diff_type_mismatch() {
        let a = parse_source(json!({"a": 1}));
        let b = parse_source(json!([1, 2]));
        
        // Type mismatch should result in a replacement of the target
        match diff(&a, &b) {
            jsondiffr::DiffValue::Replace(val) => assert_eq!(val, b),
            other => panic!("Expected DiffValue::Replace, got {:?}", other),
        }
    }

    #[test]
    fn test_diff_dict_updates() {
        let a = parse_source(json!({
            "keep": 1,
            "change": 2,
            "remove": 3
        }));
        let b = parse_source(json!({
            "keep": 1,
            "change": 22,
            "add": 4
        }));

        let d = diff(&a, &b);
        let patched = patch(&a, &d).expect("Patch should succeed");
        assert_eq!(patched, b);
    }

    #[test]
    fn test_diff_dict_similarity_zero() {
        // When similarity is 0, it should emit a patch that results in the target
        let a = parse_source(json!({"a": 1}));
        let b = parse_source(json!({"x": 10, "y": 20}));

        let d = diff(&a, &b);
        let patched = patch(&a, &d).expect("Patch should succeed");
        assert_eq!(patched, b);
    }

    #[test]
    fn test_diff_list_insertion() {
        let a = parse_source(json!([1, 2, 3]));
        let b = parse_source(json!([1, 2, 4, 3]));

        let d = diff(&a, &b);
        let patched = patch(&a, &d).expect("Patch should succeed");
        assert_eq!(patched, b);
    }

    #[test]
    fn test_diff_list_deletion() {
        let a = parse_source(json!([1, 2, 3, 4]));
        let b = parse_source(json!([1, 3, 4]));

        let d = diff(&a, &b);
        let patched = patch(&a, &d).expect("Patch should succeed");
        assert_eq!(patched, b);
    }

    #[test]
    fn test_diff_list_complex() {
        let a = parse_source(json!(['x', 'a', 'c', 'x']));
        let b = parse_source(json!(['a', 'b', 'c']));

        let d = diff(&a, &b);
        let patched = patch(&a, &d).expect("Patch should succeed");
        assert_eq!(patched, b);
    }

    #[test]
    fn test_roundtrip_property() {
        let cases = vec![
            json!({"a": 1, "b": {"c": 2}}),
            json!({"a": [1, 2, 3]}),
            json!({"a": 1, "b": 2, "c": 3}),
            json!({"a": null, "b": true}),
        ];

        for case_a_json in cases {
            let case_b_json = json!({"a": 2, "b": {"c": 3}, "d": 4});
            
            let a = parse_source(case_a_json.clone());
            let b = parse_source(case_b_json);
            
            let d = diff(&a, &b);
            let patched = patch(&a, &d).expect("Roundtrip patch failed");
            
            assert_eq!(patched, b, "Roundtrip failed for source: {:?}", case_a_json);
        }
    }
}
