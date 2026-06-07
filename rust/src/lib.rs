//! `jsondiffr` — Rust port of [jsondiff](https://github.com/xlwings/jsondiff).
//!
//! Currently provides a **patcher** that consumes the marshalled diff format
//! defined in `FORMAT.md` and applies it to an immutable JSON value. A
//! differ (i.e., `diff(source, target) -> diff`) is **not yet implemented**;
//! that will land in a future release and live alongside `patch()` here.
//!
//! # Example
//!
//! ```no_run
//! use jsondiffr::{parse_source, parse_diff, patch};
//!
//! let source: serde_json::Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
//! let wire:   serde_json::Value = serde_json::from_str(r#"{"b": 3, "$delete": ["a"]}"#).unwrap();
//!
//! let s = parse_source(source);
//! let d = parse_diff(wire).unwrap();
//! let patched = patch(&s, &d).unwrap();
//! // patched is now the immutable JsonValue equivalent of {"b": 3}
//! # let _ = patched;
//! ```
//!
//! # Scope and limitations
//!
//! * Set sources (FORMAT.md §2 "When `a` is a set") are not modelled —
//!   `JsonValue` only carries the six JSON-native types. Diffs containing
//!   `$add` or `$discard` will return `PatchError::SetSourceUnsupported`.
//! * Scalar source + non-trivial object diff returns
//!   `PatchError::ScalarSourceWithObjectDiff` rather than mirroring Python's
//!   permissive behaviour.
//! * **Differ is a TODO.** This crate cannot currently compute a diff from
//!   two values; it only applies a diff produced elsewhere (e.g., by the
//!   Python reference implementation).

mod diff;
mod differ;
mod error;
mod patcher;
mod value;

pub use diff::{parse_diff, DiffValue, Patch};
pub use differ::diff;
pub use error::{ParseError, PatchError};
pub use patcher::patch;
pub use value::{JsonNumber, JsonValue};

/// Convert a `serde_json::Value` into the immutable `JsonValue` AST without
/// applying any unmarshal rules — the source side of a patch is plain user
/// data, never a marshalled diff.
pub fn parse_source(v: serde_json::Value) -> JsonValue {
    JsonValue::from_serde(v)
}
