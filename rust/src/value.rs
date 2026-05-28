//! Immutable JSON-like value AST.
//!
//! Containers are wrapped in `Arc` so that cloning a `JsonValue` is O(1) and
//! sub-trees are shared between source and patched output. No public API
//! exposes mutable access; updates are performed by reconstructing the path
//! from the root.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diff::{escape_data_string, escape_object_key};

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

    pub fn to_serde(&self) -> serde_json::Value {
        self.to_diff_serde(false)
    }

    pub(crate) fn to_diff_serde(&self, escape_as_diff: bool) -> serde_json::Value {
        // If this JsonValue is part of a diff, we must escape $ prefixes here.
        // If this JsonValue represents a JSON source/target, we must not escape them.
        match self {
            JsonValue::Null => serde_json::Value::Null,
            JsonValue::Bool(b) => serde_json::Value::Bool(*b),
            JsonValue::Number(n) => serde_json::Value::Number(n.0.clone()),
            JsonValue::String(s) => {
                let escaped = if escape_as_diff { escape_data_string(s) } else { s.to_string() };
                serde_json::Value::String(escaped)
            }
            JsonValue::Array(a) => {
                serde_json::Value::Array(
                    a.iter().map(|v| v.to_diff_serde(escape_as_diff)).collect(),
                )
            }
            JsonValue::Object(m) => {
                let mut map = serde_json::Map::new();
                for (k, v) in m.iter() {
                    let escaped_key = if escape_as_diff { escape_object_key(k) } else { k.to_string() };
                    map.insert(escaped_key, v.to_diff_serde(escape_as_diff));
                }
                serde_json::Value::Object(map)
            }
        }
    }
}
