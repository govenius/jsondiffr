#!/usr/bin/env python3
"""Generate reference JSON fixtures for cross-language patcher testing.

Each fixture file contains a source value, a target value, and the marshalled
diff produced by the Python jsondiff implementation. A conforming patcher in
any language should satisfy:

    patch(source, diff) == target

Sets are intentionally excluded because JSON has no native set type; consumers
that need set semantics should construct hand-written test cases.

Run from the repo root with the jsondiff1 conda env active:

    python tests/generate_reference_data.py

Output: reference_data/<case_id>.json plus reference_data/_manifest.json.
"""
import json
import os
import shutil

import jsondiff


# (case_id, description, source, target)
CASES = [
    (
        "dict_add_change_delete",
        "Basic dict diff exercising all of add, change, and delete.",
        {"a": 1, "b": 2},
        {"b": 3, "c": 4},
    ),
    (
        "dict_nested_subpatch",
        "Nested dict edits propagate as sub-diffs at integer list positions.",
        {"a": [0, {"b": 4}, 1]},
        {"a": [0, {"b": 5}, 1]},
    ),
    (
        "dict_full_replace_low_similarity",
        "Disjoint keys force a $replace fallback at the top level.",
        {"a": 1, "b": 2},
        {"x": 3, "y": 4},
    ),
    (
        "dict_empty_to_populated",
        "Adding all keys to an empty dict produces $replace (similarity = 0).",
        {},
        {"a": 1, "b": 2},
    ),
    (
        "dict_populated_to_empty",
        "Removing every key produces $replace with an empty target dict.",
        {"a": 1},
        {},
    ),
    (
        "list_insert_delete_change",
        "List edit combining insertion, deletion, and an integer-keyed sub-patch.",
        ["x", "a", {"v": 11}, "c", "x"],
        ["a", {"v": 20}, "b", "c"],
    ),
    (
        "list_pure_insert",
        "Single insertion into a list.",
        ["a", "c"],
        ["a", "b", "c"],
    ),
    (
        "list_pure_delete",
        "Single deletion from a list.",
        ["a", "b", "c"],
        ["a", "c"],
    ),
    (
        "list_low_similarity_bare_target",
        "Disjoint list elements produce a bare-list replacement diff.",
        [1, 2, 3],
        ["x", "y"],
    ),
    (
        "list_of_dicts_partial_change",
        "One element changes; others remain unchanged and are omitted from the diff.",
        [{"id": 1, "v": 10}, {"id": 2, "v": 20}],
        [{"id": 1, "v": 11}, {"id": 2, "v": 20}],
    ),
    (
        "empty_diff_equal_inputs",
        "Equal inputs yield the empty-dict no-change diff.",
        {"a": [1, 2, 3]},
        {"a": [1, 2, 3]},
    ),
    (
        "replace_with_scalar_target",
        "Scalar target produces a bare value diff (no $replace wrap needed).",
        {"a": 1, "b": 2},
        42,
    ),
    (
        "replace_with_dict_target_from_list",
        "Type mismatch with a dict target is wrapped in $replace.",
        [1, 2, 3],
        {"k": "v"},
    ),
    (
        "replace_with_list_target_from_dict",
        "Type mismatch with a non-dict target stays bare.",
        {"a": 1},
        [1, 2],
    ),
    (
        "escape_dollar_prefix_key",
        "Data key starting with $ is escaped to $$ on the wire.",
        {"$key": 1},
        {"$key": 2},
    ),
    (
        "escape_dollar_prefix_value",
        "Data value with $ is escaped to $$.",
        {"key": "$value 1"},
        {"key": "value $2"},
    ),
    (
        "escape_double_dollar_prefix_key",
        "Data key starting with $$ is escaped to $$$.",
        {"$$key": 1},
        {"$$key": 2},
    ),
    (
        "null_to_value",
        "Replacing null with a scalar value.",
        None,
        42,
    ),
    (
        "value_to_null",
        "Replacing a scalar value with null.",
        42,
        None,
    ),
    (
        "deeply_nested_edit",
        "Multi-level nesting with a single change at depth 3.",
        {"a": {"b": {"c": [1, 2, 3]}}},
        {"a": {"b": {"c": [1, 9, 3]}}},
    ),
]


OUT_DIR = os.path.join(os.path.dirname(__file__), "..", "reference_data")


def main():
    out_dir = os.path.abspath(OUT_DIR)
    if os.path.exists(out_dir):
        shutil.rmtree(out_dir)
    os.makedirs(out_dir)

    differ = jsondiff.JsonDiffer()

    for case_id, desc, source, target in CASES:
        marshalled = jsondiff.diff(source, target, marshal=True)

        # Round-trip the in-memory form to catch regressions in this script.
        d_inmem = jsondiff.diff(source, target)
        patched_inmem = differ.patch(source, d_inmem)
        assert patched_inmem == target, (
            f"{case_id}: in-memory round-trip failed: {patched_inmem!r} != {target!r}"
        )

        # Round-trip the wire form to ensure JSON serialization is lossless.
        wire = json.loads(json.dumps(marshalled))
        patched_wire = differ.patch(source, differ.unmarshal(wire))
        assert patched_wire == target, (
            f"{case_id}: wire round-trip failed: {patched_wire!r} != {target!r}"
        )

        out_path = os.path.join(out_dir, f"{case_id}.json")
        with open(out_path, "w") as f:
            json.dump(
                {
                    "description": desc,
                    "source": source,
                    "target": target,
                    "diff": marshalled,
                },
                f,
                indent=2,
                sort_keys=False,
            )
            f.write("\n")

        print(f"  generated {case_id}.json")

    manifest = {
        "format_version": 1,
        "description": (
            "Reference fixtures for cross-language jsondiff patcher testing. "
            "Each <case_id>.json contains source/target/diff; conforming "
            "patchers must satisfy patch(source, diff) == target. See FORMAT.md."
        ),
        "cases": [case_id for case_id, _, _, _ in CASES],
    }
    with open(os.path.join(out_dir, "_manifest.json"), "w") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")

    print(f"\nGenerated {len(CASES)} fixtures plus _manifest.json in {out_dir}/")


if __name__ == "__main__":
    main()
