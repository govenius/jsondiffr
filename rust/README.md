# jsondiffr

Rust diff/patch implementation compliant with the JSON diff format specified in `../FORMAT.md`.

## Source files

- [src/lib.rs](src/lib.rs) — public API for both `patch()` and `diff()`.
- [src/value.rs](src/value.rs) — immutable `JsonValue` AST with `Arc`-wrapped containers.
- [src/diff.rs](src/diff.rs) — `serde_json::Value → DiffValue` parser/serializer, applying the [FORMAT.md](../FORMAT.md) §1 escaping rules.
- [src/patcher.rs](src/patcher.rs) — `patch(&source, &diff)` implementation, per [FORMAT.md](../FORMAT.md) §2.
- [src/differ.rs](src/differ.rs) — `diff(a, b)` implementation, per [FORMAT.md](../FORMAT.md) §3.
- [src/error.rs](src/error.rs) — `ParseError` and `PatchError`.

## Scope and limitations

- Set sources (FORMAT.md §2 "When `a` is a set") are not implemented —
  `JsonValue` only carries the six JSON-native types. Diffs containing
  `$add` or `$discard` will return `PatchError::SetSourceUnsupported`.
- Scalar source + non-trivial object diff returns
  `PatchError::ScalarSourceWithObjectDiff` rather than the permissible-but-not-recommended
  permissive behaviour of giving a warning only.
