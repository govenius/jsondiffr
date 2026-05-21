//! Immutable JSON-like value AST.
//!
//! Containers are wrapped in `Arc` so that cloning a `JsonValue` is O(1) and
//! sub-trees are shared between source and patched output. No public API
//! exposes mutable access; updates are performed by reconstructing the path
//! from the root.

use std::collections::BTreeMap;
use std::sync::Arc;

/// Newtype around `serde_json::Number` so we can derive `Eq` cleanly and have
/// a stable comparison point for the rest of the crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonNumber(pub serde_json::Number);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(Arc<str>),
    Array(Arc<Vec<JsonValue>>),
    Object(Arc<BTreeMap<Arc<str>, JsonValue>>),
    // NOTE: A `Set` variant is omitted. JSON has no native set type, so a
    // patcher that only ingests JSON cannot reconstruct one. If/when a
    // differ is added that supports sets, this enum should grow a
    // `Set(Arc<BTreeSet<JsonValue>>)` variant and the patch dispatch will
    // need a corresponding branch.
}

impl JsonValue {
    pub fn from_serde(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => JsonValue::Null,
            serde_json::Value::Bool(b) => JsonValue::Bool(b),
            serde_json::Value::Number(n) => JsonValue::Number(JsonNumber(n)),
            serde_json::Value::String(s) => JsonValue::String(Arc::from(s)),
            serde_json::Value::Array(a) => JsonValue::Array(Arc::new(
                a.into_iter().map(JsonValue::from_serde).collect(),
            )),
            serde_json::Value::Object(m) => JsonValue::Object(Arc::new(
                m.into_iter()
                    .map(|(k, v)| (Arc::from(k), JsonValue::from_serde(v)))
                    .collect(),
            )),
        }
    }
}
