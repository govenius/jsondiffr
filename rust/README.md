# jsondiffr

Rust port of [jsondiff](https://github.com/xlwings/jsondiff). The crate name reflects the planned scope — **patcher today, differ later**.

[Cargo.toml](Cargo.toml) declares `serde_json` and `thiserror` as the only dependencies.

## Source files

- [src/lib.rs](src/lib.rs) — public API plus a doc comment noting the differ is a TODO placeholder.
- [src/value.rs](src/value.rs) — immutable `JsonValue` AST with `Arc`-wrapped containers. The `Set` variant is omitted; a comment marks where it would slot in when a future differ adds set support.
- [src/error.rs](src/error.rs) — `ParseError` and `PatchError`.
- [src/diff.rs](src/diff.rs) — `serde_json::Value → DiffValue` parser applying the [FORMAT.md](../FORMAT.md) §1 unescape rules.
- [src/patch.rs](src/patch.rs) — `patch(&source, &diff)` implementing FORMAT.md §2 with dict/list dispatch, the descending-delete → insert → integer-keyed-sub-patch ordering, and a `patch_as_data` helper for the rare addition-of-dict-shape edge case.

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

- **Differ not implemented.** This crate cannot currently compute a diff from two values; it only applies a diff produced by the Python reference (or any conforming differ). A Rust differ is planned for a future release and will live alongside `patch()` in this crate.
- **Set sources unsupported.** JSON has no native set type, so a JSON-only patcher cannot reconstruct one. Diffs containing `$add` or `$discard` return `PatchError::SetSourceUnsupported`.
- **Scalar source + non-trivial object diff returns an error** rather than mirroring Python's permissive behaviour.
