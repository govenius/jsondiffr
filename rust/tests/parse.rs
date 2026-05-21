//! Unit tests for the parser / unmarshal logic (FORMAT.md §1).
//!
//! Mirrors the §1 marshalled-format group plus the bare-`delete`-key
//! cross-cutting test from `TestFormatConformance` in test_jsondiff.py.

use jsondiffr::{parse_diff, parse_source, patch, DiffValue};
use serde_json::json;

#[test]
fn unescape_chain_strips_one_dollar() {
    // "$$x" on the wire decodes to a literal "$x" entry key.
    let source = parse_source(json!({"$x": 0}));
    let diff = parse_diff(json!({"$$x": 1})).unwrap();
    assert_eq!(patch(&source, &diff).unwrap(), parse_source(json!({"$x": 1})));

    // "$$$x" decodes to "$$x".
    let source = parse_source(json!({"$$x": 0}));
    let diff = parse_diff(json!({"$$$x": 1})).unwrap();
    assert_eq!(
        patch(&source, &diff).unwrap(),
        parse_source(json!({"$$x": 1}))
    );
}

#[test]
fn bare_delete_string_is_data() {
    let parsed = parse_diff(json!({"delete": 2})).unwrap();
    let DiffValue::Patch(p) = parsed else {
        panic!("expected DiffValue::Patch, got {parsed:?}");
    };
    assert!(
        p.delete.is_none(),
        "the literal string \"delete\" must not be classified as the $delete sentinel"
    );
    assert!(
        p.entries.contains_key("delete"),
        "expected \"delete\" to appear as a data entry"
    );
}

#[test]
fn deep_nesting_does_not_overflow() {
    // Build a 100-level-deep dict source and a matching sub-patch diff
    // that descends to the bottom and updates the leaf.
    const DEPTH: usize = 100;

    let mut src = json!(0);
    let mut dif = json!(1);
    let mut tgt = json!(1);
    for _ in 0..DEPTH {
        src = json!({"k": src});
        dif = json!({"k": dif});
        tgt = json!({"k": tgt});
    }

    let s = parse_source(src);
    let d = parse_diff(dif).unwrap();
    let t = parse_source(tgt);
    assert_eq!(patch(&s, &d).unwrap(), t);
}
