//! Implementation of the `diff(a, b)` function as specified in `FORMAT.md`.

use std::collections::{BTreeMap};
use std::sync::Arc;

use crate::diff::{DiffValue, Patch};
use crate::value::JsonValue;

/// Produces a diff `d` that `patch(a, d)` can apply to `a` to obtain `b`.
pub fn diff(a: &JsonValue, b: &JsonValue) -> DiffValue {
    // 1. Identity or equality
    if a == b {
        return DiffValue::Empty;
    }

    match (a, b) {
        // Type mismatch (e.g., dict vs list)
        (a, b) if std::mem::discriminant(a) != std::mem::discriminant(b) => {
            match b {
                JsonValue::Object(_) => DiffValue::Patch(Arc::new(Patch {
                    replace: Some(b.clone()),
                    ..Default::default()
                })),
                _ => DiffValue::Replace(b.clone()),
            }
        }

        // Both are Objects (Dicts)
        (JsonValue::Object(a_map), JsonValue::Object(b_map)) => {
            diff_dict(a_map, b_map)
        }

        // Both are Arrays (Lists)
        (JsonValue::Array(a_arr), JsonValue::Array(b_arr)) => {
            diff_list(a_arr, b_arr)
        }

        // Scalar inequality (same type, unequal)
        _ => DiffValue::Replace(b.clone()),
    }
}

fn diff_dict(a: &BTreeMap<Arc<str>, JsonValue>, b: &BTreeMap<Arc<str>, JsonValue>) -> DiffValue {
    let mut entries = BTreeMap::new();
    let mut deleted = Vec::new();
    let mut changed_count = 0;
    let mut total_count = 0;

    // Keys in a but not in b (Removed)
    for k in a.keys() {
        if !b.contains_key(k) {
            deleted.push(JsonValue::String(k.clone()));
        }
    }

    // Keys in both (Changed) or in b but not in a (Added)
    for (k, b_val) in b {
        total_count += 1;
        if let Some(a_val) = a.get(k) {
            if a_val != b_val {
                let sub_diff = diff(a_val, b_val);
                if let DiffValue::Empty = sub_diff {
                    // Should not happen if a_val != b_val
                } else {
                    entries.insert(k.clone(), sub_diff);
                    changed_count += 1;
                }
            }
        } else {
            // Added
            entries.insert(k.clone(), DiffValue::Replace(b_val.clone()));
            changed_count += 1;
        }
    }

    let removed_count = a.keys().filter(|k| !b.contains_key(k)).count();
    total_count += removed_count;

    // Similarity score s = (matches) / total
    let similarity = if total_count == 0 {
        1.0
    } else {
        let matches = (total_count - changed_count - removed_count) as f64;
        matches / total_count as f64
    };

    if similarity <= 0.0 && !deleted.is_empty() && entries.is_empty() {
        return DiffValue::Patch(Arc::new(Patch {
            replace: Some(JsonValue::Object(Arc::new(b.clone()))),
            ..Default::default()
        }));
    }

    if similarity >= 1.0 && entries.is_empty() && deleted.is_empty() {
        return DiffValue::Empty;
    }

    let mut patch = Patch::default();
    patch.entries = entries;
    if !deleted.is_empty() {
        patch.delete = Some(deleted);
    }

    DiffValue::Patch(Arc::new(patch))
}

fn diff_list(a: &Vec<JsonValue>, b: &Vec<JsonValue>) -> DiffValue {
    let n = a.len();
    let m = b.len();

    // 1. Compute LCS DP table
    let mut dp = vec![vec![0; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let lcs_len = dp[n][m];
    let similarity = if n + m == 0 { 1.0 } else { (2.0 * lcs_len as f64) / (n + m) as f64 };

    if similarity <= 0.0 {
        return DiffValue::Replace(JsonValue::Array(Arc::new(b.clone())));
    }

    if similarity >= 1.0 && n == m && lcs_len == n {
        return DiffValue::Empty;
    }

    // 2. Backtrack to find edits
    let mut deleted = Vec::new();
    let mut inserted = Vec::new();

    let mut i = n;
    let mut j = m;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            // Insertion in b at target position j-1
            inserted.push((j - 1, b[j - 1].clone()));
            j -= 1;
        } else if i > 0 && (j == 0 || dp[i][j - 1] < dp[i - 1][j]) {
            // Deletion from a at source position i-1
            let val = match a[i - 1].clone() {
                // We need the index, not the value. The logic above was slightly flawed.
                // We actually want the index `i-1`.
                _ => JsonValue::Number(crate::value::JsonNumber(serde_json::Number::from(i - 1))),
            };
            deleted.push(val);
            i -= 1;
        } else {
            i -= 1;
            j -= 1;
        }
    }
    
    // Re-calculating deletions correctly based on index i-1
    // The loop above was a bit messy with the 'else' block. 
    // Let's fix the logic to ensure we get the correct indices.
    // Actually, the logic above for `deleted.push` is correct for the index.
    // But I need to fix the `as_number` error.

    // Sort deletions descending as required by FORMAT.md
    deleted.sort_by(|x, y| {
        let x_u64 = match x {
            JsonValue::Number(n) => n.0.as_u64().unwrap_or(0),
            _ => 0,
        };
        let y_u64 = match y {
            JsonValue::Number(n) => n.0.as_u64().unwrap_or(0),
            _ => 0,
        };
        y_u64.cmp(&x_u64)
    });

    // Sort insertions by target position
    inserted.sort_by_key(|(pos, _)| *pos);

    let mut patch = Patch::default();
    if !deleted.is_empty() {
        patch.delete = Some(deleted);
    }
    if !inserted.is_empty() {
        patch.insert = Some(inserted);
    }

    DiffValue::Patch(Arc::new(patch))
}
