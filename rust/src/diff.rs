//! Parse a `serde_json::Value` into the immutable `DiffValue` AST,
//! applying the unmarshal rules from FORMAT.md §1.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value as Sj;

use crate::error::ParseError;
use crate::value::{JsonNumber, JsonValue};

// ── Sentinel labels (§1) ───────────────────────────────────────────────────
const REPLACE: &str = "$replace";
const DELETE: &str = "$delete";
const INSERT: &str = "$insert";
const ADD: &str = "$add";
const DISCARD: &str = "$discard";

#[derive(Clone, Debug, PartialEq)]
pub enum DiffValue {
    /// §2 rule 1: diff was not a JSON object — wholesale replacement.
    Replace(JsonValue),
    /// §2 rule 2: diff was the empty object `{}` — no change.
    Empty,
    /// Diff was a non-empty object. May contain `$replace` (§2 rule 3) or
    /// any combination of non-sentinel keys and the structural sentinels.
    Patch(Arc<Patch>),
}

impl DiffValue {
    /// Converts the `DiffValue` into a `serde_json::Value` following the
    /// marshaling rules in FORMAT.md.
    pub fn to_serde(&self) -> Sj {
        match self {
            DiffValue::Empty => Sj::Object(serde_json::Map::new()),
            DiffValue::Replace(v) => {
                let mut map = serde_json::Map::new();
                map.insert(REPLACE.to_string(), v.to_serde(true));
                Sj::Object(map)
            }
            DiffValue::Patch(p) => {
                let mut map = serde_json::Map::new();

                if let Some(replace_val) = &p.replace {
                    map.insert(REPLACE.to_string(), replace_val.to_serde(true));
                }

                if let Some(delete_vals) = &p.delete {
                    let mut del_arr = Vec::new();
                    for v in delete_vals {
                        del_arr.push(v.to_serde(true));
                    }
                    map.insert(DELETE.to_string(), Sj::Array(del_arr));
                }

                if let Some(insert_vals) = &p.insert {
                    let mut ins_arr = Vec::new();
                    for (pos, val) in insert_vals {
                        let mut pair = Vec::new();
                        pair.push(Sj::from(*pos));
                        pair.push(val.to_serde(true));
                        ins_arr.push(Sj::Array(pair));
                    }
                    map.insert(INSERT.to_string(), Sj::Array(ins_arr));
                }

                if let Some(add_vals) = &p.add {
                    let mut add_arr = Vec::new();
                    for v in add_vals {
                        add_arr.push(v.to_serde(true));
                    }
                    map.insert(ADD.to_string(), Sj::Array(add_arr));
                }

                if let Some(discard_vals) = &p.discard {
                    let mut disc_arr = Vec::new();
                    for v in discard_vals {
                        disc_arr.push(v.to_serde(true));
                    }
                    map.insert(DISCARD.to_string(), Sj::Array(disc_arr));
                }

                for (key, entry) in p.entries.iter() {
                    // Marshal the key: escape $ if it's a data key.
                    // According to FORMAT.md §1: "A string "$x" already starting with $ -> "$$x""
                    map.insert(escape_object_key(key), entry.to_serde());
                }

                Sj::Object(map)
            }
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Patch {
    pub replace: Option<JsonValue>,
    /// Non-sentinel object keys mapped to their sub-diffs. For dict sources
    /// these are data keys; for list sources, the keys are parsed as `usize`
    /// positions at apply time.
    pub entries: BTreeMap<Arc<str>, DiffValue>,
    /// `$delete` payload (dict keys *or* list positions, depending on source).
    pub delete: Option<Vec<JsonValue>>,
    /// `$insert` payload (list sources only).
    pub insert: Option<Vec<(usize, JsonValue)>>,
    /// `$add` payload (set sources only — unsupported in this crate).
    pub add: Option<Vec<JsonValue>>,
    /// `$discard` payload (set sources only — unsupported in this crate).
    pub discard: Option<Vec<JsonValue>>,
}

// ── Public entry point ─────────────────────────────────────────────────────

pub fn parse_diff(v: Sj) -> Result<DiffValue, ParseError> {
    match v {
        Sj::Object(m) if m.is_empty() => Ok(DiffValue::Empty),
        Sj::Object(m) => Ok(DiffValue::Patch(Arc::new(parse_patch_object(m)?))),
        other => Ok(DiffValue::Replace(parse_diff_value(other)?)),
    }
}

// ── Object parsing: classify each key, dispatch ────────────────────────────

fn parse_patch_object(m: serde_json::Map<String, Sj>) -> Result<Patch, ParseError> {
    let mut p = Patch::default();
    for (k, v) in m {
        match k.as_str() {
            REPLACE => p.replace = Some(parse_diff_value(v)?),
            DELETE => p.delete = Some(parse_array_of_values(v, ParseError::MalformedDelete)?),
            INSERT => p.insert = Some(parse_insert_array(v)?),
            ADD => p.add = Some(parse_array_of_values(v, ParseError::MalformedSet)?),
            DISCARD => p.discard = Some(parse_array_of_values(v, ParseError::MalformedSet)?),
            _ => {
                let key = unescape_object_key(&k);
                p.entries.insert(key, parse_diff(v)?);
            }
        }
    }
    Ok(p)
}

/// FORMAT.md §1: "$$x" → "$x"; unknown "$<label>" → strip a single `$`;
/// otherwise unchanged.
fn unescape_object_key(k: &str) -> Arc<str> {
    if let Some(rest) = k.strip_prefix('$') {
        Arc::from(rest)
    } else {
        Arc::from(k)
    }
}

/// Inverse of `unescape_object_key`.
pub(crate) fn escape_object_key(k: &str) -> String {
    if k.starts_with('$') {
        format!("${}", k)
    } else {
        k.to_string()
    }
}

// ── Value parsing inside a diff (recursive string/object key unescape) ─────

/// Parse a JSON value that appears as *data* inside a diff (the `$replace`
/// value, an inserted element, a `$delete` key, an addition value, etc.).
/// Recurses into containers and applies the string unescape rule.
fn parse_diff_value(v: Sj) -> Result<JsonValue, ParseError> {
    Ok(match v {
        Sj::Null => JsonValue::Null,
        Sj::Bool(b) => JsonValue::Bool(b),
        Sj::Number(n) => JsonValue::Number(JsonNumber(n)),
        Sj::String(s) => JsonValue::String(unescape_data_string(&s)),
        Sj::Array(a) => JsonValue::Array(Arc::new(
            a.into_iter()
                .map(parse_diff_value)
                .collect::<Result<_, _>>()?,
        )),
        Sj::Object(m) => JsonValue::Object(Arc::new(
            m.into_iter()
                .map(|(k, v)| Ok((unescape_object_key(&k), parse_diff_value(v)?)))
                .collect::<Result<_, ParseError>>()?,
        )),
    })
}

fn unescape_data_string(s: &str) -> Arc<str> {
    if let Some(rest) = s.strip_prefix('$') {
        Arc::from(rest)
    } else {
        Arc::from(s)
    }
}

/// Inverse of `unescape_data_string`.
pub(crate) fn escape_data_string(s: &str) -> String {
    if s.starts_with('$') {
        format!("${}", s)
    } else {
        s.to_string()
    }
}

// ── Sentinel payload parsers ───────────────────────────────────────────────

fn parse_array_of_values(v: Sj, err: ParseError) -> Result<Vec<JsonValue>, ParseError> {
    let arr = v.as_array().cloned().ok_or(err)?;
    arr.into_iter().map(parse_diff_value).collect()
}

fn parse_insert_array(v: Sj) -> Result<Vec<(usize, JsonValue)>, ParseError> {
    let arr = v.as_array().cloned().ok_or(ParseError::MalformedInsert)?;
    arr.into_iter()
        .map(|elem| {
            let pair = elem.as_array().cloned().ok_or(ParseError::MalformedInsert)?;
            if pair.len() != 2 {
                return Err(ParseError::MalformedInsert);
            }
            let mut it = pair.into_iter();
            let pos_value = it.next().unwrap();
            let element = it.next().unwrap();
            let pos_i64 = pos_value
                .as_i64()
                .ok_or(ParseError::MalformedInsert)?;
            let pos = usize::try_from(pos_i64)
                .map_err(|_| ParseError::PositionOutOfRange(pos_i64))?;
            Ok((pos, parse_diff_value(element)?))
        })
        .collect()
}
