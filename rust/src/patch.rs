//! Apply a `DiffValue` to an immutable `JsonValue` source, producing a new
//! `JsonValue`. Implements FORMAT.md §2.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::diff::{DiffValue, Patch};
use crate::error::PatchError;
use crate::value::JsonValue;

pub fn patch(source: &JsonValue, diff: &DiffValue) -> Result<JsonValue, PatchError> {
    match diff {
        DiffValue::Replace(v) => Ok(v.clone()),       // §2 rule 1
        DiffValue::Empty => Ok(source.clone()),       // §2 rule 2
        DiffValue::Patch(p) => apply_patch(source, p),
    }
}

fn apply_patch(source: &JsonValue, p: &Patch) -> Result<JsonValue, PatchError> {
    // §2 rule 3: $replace short-circuits everything else.
    if let Some(r) = &p.replace {
        return Ok(r.clone());
    }
    // $add/$discard imply a set source, which this implementation doesn't
    // model. Reject them explicitly rather than silently ignoring.
    if p.add.is_some() || p.discard.is_some() {
        return Err(PatchError::SetSourceUnsupported);
    }
    // §2 rule 4: dispatch on source type.
    match source {
        JsonValue::Object(m) => apply_dict(m, p),
        JsonValue::Array(a) => apply_list(a, p),
        // Scalars: a non-replace, non-empty object diff has no defined
        // semantics here. (Python returns the raw dict; for safety we error.)
        _ => Err(PatchError::ScalarSourceWithObjectDiff),
    }
}

// ── Dict source ────────────────────────────────────────────────────────────

fn apply_dict(
    source: &Arc<BTreeMap<Arc<str>, JsonValue>>,
    p: &Patch,
) -> Result<JsonValue, PatchError> {
    let mut out: BTreeMap<Arc<str>, JsonValue> = (**source).clone();

    // Apply sub-patches / additions.
    for (k, sub) in &p.entries {
        match out.get(k) {
            None => {
                // Key absent: the sub-diff is treated as raw data (insertion).
                out.insert(k.clone(), diff_value_as_data(sub));
            }
            Some(existing) => {
                let patched = patch(existing, sub)?;
                out.insert(k.clone(), patched);
            }
        }
    }

    // Apply $delete.
    if let Some(keys) = &p.delete {
        for key_val in keys {
            let key = match key_val {
                JsonValue::String(s) => s.clone(),
                _ => return Err(PatchError::DictDeleteNotString),
            };
            if out.remove(&key).is_none() {
                return Err(PatchError::DictDeleteMissingKey(key));
            }
        }
    }

    Ok(JsonValue::Object(Arc::new(out)))
}

// ── List source ────────────────────────────────────────────────────────────

fn apply_list(source: &Arc<Vec<JsonValue>>, p: &Patch) -> Result<JsonValue, PatchError> {
    let mut out: Vec<JsonValue> = (**source).clone();

    // 1. $delete first, source-indexed, in given (descending) order.
    if let Some(positions) = &p.delete {
        for pos_val in positions {
            let pos = expect_usize(pos_val).ok_or(PatchError::ListDeleteNotInteger)?;
            if pos >= out.len() {
                return Err(PatchError::ListIndexOutOfRange(pos, out.len()));
            }
            out.remove(pos);
        }
    }

    // 2. $insert, applied to the post-delete list in trace order.
    if let Some(pairs) = &p.insert {
        for (pos, value) in pairs {
            if *pos > out.len() {
                return Err(PatchError::ListIndexOutOfRange(*pos, out.len()));
            }
            out.insert(*pos, value.clone());
        }
    }

    // 3. Integer-keyed sub-patches index the final list.
    for (k, sub) in &p.entries {
        let idx: usize = k
            .parse()
            .map_err(|_| PatchError::InvalidListPosition(k.clone()))?;
        if idx >= out.len() {
            return Err(PatchError::ListIndexOutOfRange(idx, out.len()));
        }
        let patched = patch(&out[idx], sub)?;
        out[idx] = patched;
    }

    Ok(JsonValue::Array(Arc::new(out)))
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn expect_usize(v: &JsonValue) -> Option<usize> {
    match v {
        JsonValue::Number(n) => n
            .0
            .as_i64()
            .and_then(|i| usize::try_from(i).ok())
            .or_else(|| n.0.as_u64().and_then(|u| usize::try_from(u).ok())),
        _ => None,
    }
}

/// Convert a sub-diff into its raw JsonValue form. Used when applying an
/// "insertion" into a dict source (the key was absent, so the sub-diff is
/// taken at face value as the new value).
fn diff_value_as_data(d: &DiffValue) -> JsonValue {
    match d {
        DiffValue::Replace(v) => v.clone(),
        DiffValue::Empty => JsonValue::Object(Arc::new(BTreeMap::new())),
        DiffValue::Patch(p) => patch_as_data(p),
    }
}

/// Reconstruct a JsonValue::Object from a Patch when the Patch is being used
/// as an insertion target. Sentinels are re-marshalled to their string form
/// so the resulting object looks like the original wire data.
fn patch_as_data(p: &Patch) -> JsonValue {
    let mut m: BTreeMap<Arc<str>, JsonValue> = BTreeMap::new();

    if let Some(r) = &p.replace {
        m.insert(Arc::from("$replace"), r.clone());
    }
    if let Some(d) = &p.delete {
        m.insert(Arc::from("$delete"), array(d.iter().cloned()));
    }
    if let Some(i) = &p.insert {
        m.insert(Arc::from("$insert"), insert_to_array(i));
    }
    if let Some(a) = &p.add {
        m.insert(Arc::from("$add"), array(a.iter().cloned()));
    }
    if let Some(d) = &p.discard {
        m.insert(Arc::from("$discard"), array(d.iter().cloned()));
    }
    for (k, sub) in &p.entries {
        m.insert(k.clone(), diff_value_as_data(sub));
    }
    JsonValue::Object(Arc::new(m))
}

fn array<I: IntoIterator<Item = JsonValue>>(it: I) -> JsonValue {
    JsonValue::Array(Arc::new(it.into_iter().collect()))
}

fn insert_to_array(pairs: &[(usize, JsonValue)]) -> JsonValue {
    let arr: Vec<JsonValue> = pairs
        .iter()
        .map(|(p, v)| {
            JsonValue::Array(Arc::new(vec![
                JsonValue::Number(crate::value::JsonNumber(serde_json::Number::from(*p))),
                v.clone(),
            ]))
        })
        .collect();
    JsonValue::Array(Arc::new(arr))
}
