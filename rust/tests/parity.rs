//! Parity tests for basic equality and identity cases from `test_jsondiff.py`.

use jsondiffr::{diff, parse_source};
use serde_json::json;

#[test]
fn test_basic_identities() {
    let cases = [
        (json!(1), json!(1)),
        (json!(true), json!(true)),
        (json!("abc"), json!("abc")),
        (json!([1, 2]), json!([1, 2])),
        (json!({}), json!({})),
        (json!(null), json!(null)),
    ];

    for (a_json, b_json) in cases {
        let a = parse_source(a_json);
        let b = parse_source(b_json);
        let d = diff(&a, &b);
        match d {
            jsondiffr::DiffValue::Empty => {}
            _ => panic!("Expected Empty diff for {:?} vs {:?}, got {:?}", a_json, b_json, d),
        }
    }
}

#[test]
fn test_basic_replacements() {
    let cases = [
        (json!(1), json!(2)),
        (json!(true), json!(false)),
        (json!("abc"), json!("def")),
        (json!([1, 2]), json!([3, 4])),
    ];

    for (a_json, b_json) in cases {
        let a = parse_source(a_json);
        let b = parse_source(b_json);
        let d = diff(&a, &b);
        match d {
            jsondiffr::DiffValue::Replace(val) => assert_eq!(val, b),
            _ => panic!("Expected Replace diff for {:?} vs {:?}, got {:?}", a_json, b_json, d),
        }
    }
}
