# jsondiffr

Rust implementation compliant with the JSON diff format specified in `FORMAT.md`. Provides a **patcher** that consumes the marshalled diff format defined in `FORMAT.md` and a **differ** that computes the difference between two JSON structures.

## Usage

### Rust

```rust
use jsondiffr::{diff, patch, parse_source, parse_diff};
use serde_json::json;

fn main() {
    let a = parse_source(json!({"name": "Alice", "skills": ["Python"]}));
    let b = parse_source(json!({"name": "Alice", "skills": ["Python", "Rust"]}));

    // Compute a diff
    let d = diff(&a, &b);

    // Apply the diff to a
    let patched = patch(&a, &d).unwrap();

    assert_eq!(patched, b);
}
```

### Python

```python
import jsondiffr

a = {"name": "Alice", "skills": ["Python"]}
b = {"name": "Alice", "skills": ["Python", "Rust"]}

# Compute a diff
d = jsondiffr.diff(a, b)

# Apply the diff to a
patched = jsondiffr.patch(a, d)

assert patched == b
```

## Source files

- [src/lib.rs](src/lib.rs) — public API for both `patch()` and `diff()`.
- [src/value.rs](src/value.rs) — immutable `JsonValue` AST with `Arc`-wrapped containers.
- [src/diff.rs](src/diff.rs) — `serde_json::Value → DiffValue` parser/serializer, applying the [FORMAT.md](../FORMAT.md) §1 escaping rules.
- [src/patcher.rs](src/patcher.rs) — `patch(&source, &diff)` implementation, per [FORMAT.md](../FORMAT.md) §2.
- [src/differ.rs](src/differ.rs) — `diff(a, b)` implementation, per [FORMAT.md](../FORMAT.md) §3.
- [src/error.rs](src/error.rs) — `ParseError` and `PatchError`.

## Tests

[tests/reference.rs](tests/reference.rs) reads [../reference_data/_manifest.json](../reference_data/_manifest.json), loads each fixture, and asserts:

```text
patch(parse_source(source), parse_diff(diff)) == parse_source(target)
```

Run the suite:

```bash
cargo test --test reference
```

## Scope and limitations

- Set sources (FORMAT.md §2 "When `a` is a set") are not implemented —
  `JsonValue` only carries the six JSON-native types. Diffs containing
  `$add` or `$discard` will return `PatchError::SetSourceUnsupported`.
- Scalar source + non-trivial object diff returns
  `PatchError::ScalarSourceWithObjectDiff` rather than the permissible-but-not-recommended
  permissive behaviour of giving a warning only.
