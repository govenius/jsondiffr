# jsondiffr

Robust and stable diffs of pure JSON files.

## Goals

The diff format produced and consumed by `jsondiffr` is aimed to be
**stable** and **well-specified**, so that the diffs are suitable for
long-term storage. The format is also **compact**.

The diff format matches that of
[jsondiff](https://github.com/xlwings/jsondiff) (v2.2.1), except that
it is somewhat **reduced** in terms of functionality:
- Only pure JSON is supported. Python sets are not supported.
- Only the "compact" format is supported.
- The escape character for sentinels is hardcoded as `$`.

## Format specification (FORMAT.md)

See `FORMAT.md` for a detailed language-agnostic specification of the
diff format.

## Rust Implementation

The core implementation is under `rust/`.

## Python bindings

A separate crate under `bindings/python` provides bindings to Python.

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

## Reference Python reference implementation

We aim to have `patch(source, d)` correctly apply diffs generated with
[jsondiff](https://github.com/xlwings/jsondiff) v2.2.1.

For reference and for generating reference data sets for testing, we
store a reduced copy of v2.2.1 under `jsondiff_legacy_reference/`.  It
is reduced in that it supports only the `compact` format and escaping
is hardcoded to `$`.

## Tests

### Rust

``cargo test --manifest-path rust/Cargo.toml``

### Python bindings

```console
cd bindings/python/
maturin develop
python -m pytest tests/test_jsondiffr.py
```

### Benchmarking

You can do some very rough benchmarking using `python benchmark/benchmark.py`.

It assumes you have installed the `jsondiffr` Python bindings *and jsondiff*.
