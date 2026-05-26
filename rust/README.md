# jsondiffr

Rust port of [jsondiff](https://github.com/xlwings/jsondiff). Provides a **patcher** that consumes the marshalled diff format defined in `FORMAT.md` and a **differ** that computes the difference between two JSON structures.

## Source files

- [src/lib.rs](src/lib.rs) — public API for both `patch()` and `diff()`.
- [src/value.rs](src/value.rs) — immutable `JsonValue` AST with `Arc`-wrapped containers. The `Set` variant is omitted; a comment marks where it would slot in when a future differ adds set support.
- [src/error.rs](src/error.rs) — `ParseError` and `PatchError`.
- [src/diff.rs](src/diff.rs) — `serde_json::Value → DiffValue` parser applying the [FORMAT.md](../FORMAT.md) §1 unescape rules.
- [src/patch.rs](src/patch.rs) — `patch(&source, &diff)` implementing FORMAT.md §2 with dict/list dispatch, the descending-delete → insert → integer-keyed-sub-patch ordering, and a `patch_as_data` helper for the rare addition-of-dict-shape edge case.
- [src/differ.rs](src/differ.rs) — `diff(a, b)` implementation.

## Tests

[tests/reference.rs](tests/reference.rs) reads [../reference_data/_manifest.json](../reference_data/_manifest.json), loads each fixture, and asserts:

```text
patch(parse_source(source), parse_diff(diff)) == parse_source(target)
```

Run the suite:

```bash
cargo test --test reference
```

The current suite reports `19 reference fixtures passed`. `cargo clippy --all-targets -- -D warnings` is clean.

## Scope and limitations

- Set sources (FORMAT.md §2 "When `a` is a set") are not modelled —
  `JsonValue` only carries the six JSON-native types. Diffs containing
  `$add` or `$discard` will return `PatchError::SetSourceUnsupported`.
- Scalar source + non-trivial object diff returns
  `PatchError::ScalarSourceWithObjectDiff` rather than mirroring Python's
  permissive behaviour.
