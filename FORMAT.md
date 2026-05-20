# Diff Format

This document specifies the diff format produced by `jsondiff.diff(a, b)` and consumed by `jsondiff.patch(a, d)`. The format is designed to be small (it omits unchanged data) and to round-trip back to the modified structure via `patch`.

For any source `a` and target `b`, `patch(a, diff(a, b)) == b`.

## 1. Representation

A diff is itself a JSON-like value: a `dict`, `list`, `set`, scalar, or `None`. The *shape* of the diff is determined by the **source** value at that position, not by the target.

### Sentinels

Six special keys mark structural operations. In memory they are instances of `jsondiff.Symbol`; in the marshaled (string-encoded) form they appear as `$<label>`. The escape prefix `$` is configurable via `escape_str=`.

| In-memory       | Marshaled    | Used in          | Meaning                               |
| --------------- | ------------ | ---------------- | ------------------------------------- |
| `replace`       | `$replace`   | any container    | Wholesale replacement of the value    |
| `delete`        | `$delete`    | dict / list      | Remove keys (dict) or positions (list)|
| `insert`        | `$insert`    | list             | Insert `(position, value)` pairs      |
| `add`           | `$add`       | set              | Add elements                          |
| `discard`       | `$discard`   | set              | Remove elements                       |

One additional Symbol, `missing`, is exported but used only as an internal sentinel inside the differ and patcher — it never appears in a diff.

### The empty diff

An empty dict `{}` at any level means **no change**. `patch(a, {}) == a`.

## 2. Diff shapes by source type

The shape of a diff `d` depends on `type(a)`, where `a` is the value being patched. There are five cases.

### 2.1 Both sides are dicts

```
d = {
    <added-or-changed-key>: <sub-diff or new value>,
    ...,
    delete: [<removed-key>, ...]   # optional
}
```

- A key that is **added** (in `b` but not in `a`) maps directly to its new value.
- A key that is **changed** (in both, but different) maps to a recursive sub-diff computed against the original sub-value.
- A key that is **removed** (in `a` but not in `b`) is listed under the `delete` sentinel.
- An **unchanged** key is omitted entirely.

If a key appears at the top level of the diff and you cannot tell whether it represents an insertion or a sub-patch, the patcher decides by checking whether that key exists in `a`: missing → use the diff value as the new value; present → recurse.

### 2.2 Both sides are lists or tuples

```
d = {
    <int-position>: <sub-diff>,    # 0 or more changed elements
    ...,
    delete: [<source-position>, ...],          # optional, descending order
    insert: [(<target-position>, <value>), ...]  # optional
}
```

- `delete` lists positions in the **source list**, in **descending order**. The patcher pops them sequentially, so the order matters.
- `insert` is a sequence of `(position, value)` pairs applied to the post-delete list in the order given. Positions refer to the resulting list at the moment of insertion.
- Numeric keys are positions in the **final list** (after deletes and inserts) that point at elements needing a recursive sub-patch.

Tuples are preserved: if the source is a tuple, the patched value is also a tuple.

### 2.3 Both sides are sets

```
d = {
    add: {<element>, ...},      # optional
    discard: {<element>, ...},  # optional
}
```

`add` and `discard` are themselves sets. They are not JSON-serializable as-is; the consumer must convert them to lists (or use a custom encoder) before serializing.

### 2.4 Both sides are scalars (or `None`, or unequal types)

```
d = <b>                  # if b is not a dict
d = {replace: <b>}       # if b is a dict
```

The diff *is* the new value `b`. This case is also used when `a` and `b` have different types (e.g., `dict` vs `list`).

### 2.5 The "low similarity" fallback (containers)

If a recursive container diff would have to delete and re-insert nearly every element, the differ falls back to wholesale replacement using the same rule as 2.4: the diff is `b` itself (if `b` is not a dict) or `{replace: b}` (if `b` is a dict).

In particular:

- `diff([], <non-empty list>) → <non-empty list>` (the target list itself).
- `diff(<non-empty list>, []) → []` (the empty target list).
- `diff({a: 1}, {b: 2}) → {replace: {b: 2}}` — wrapped because the target is a dict, and an unwrapped `{b: 2}` would be ambiguous with a dict-patch.

## 3. Applying a diff

The patch algorithm is fully determined by the source `a` and the diff `d`:

1. If `d` is **not a dict**, return `d` (this is the replacement case).
2. If `d` is the **empty dict**, return `a` unchanged.
3. If `d` is a dict and contains the `replace` sentinel, return `d[replace]`.
4. Otherwise dispatch on `type(a)`:
   - **`a` is a dict**: copy `a`, then for each `(k, v)` in `d`:
     - if `k is delete`: for each `key in v`, delete it from `a`;
     - else if `k` is absent from `a`: insert `a[k] = v` verbatim;
     - else: recurse — `a[k] = patch(a[k], v)`.
   - **`a` is a list or tuple**: copy `a` into a list, then:
     - if `delete in d`: pop each position in `d[delete]` (descending order assumed);
     - if `insert in d`: insert each `(pos, value)` in order;
     - for every other (integer) key in `d`: `a[int(k)] = patch(a[int(k)], v)`.
     - Restore the original container type (tuple → tuple).
   - **`a` is a set**: copy `a`, discard each element in `d[discard]`, add each element in `d[add]`.
5. If `a` is a scalar (and `d` was a dict that didn't match any of the above), return `d`.

## 4. Marshaling

`JsonDiffer.marshal(d)` converts an in-memory diff to a fully JSON-compatible (modulo sets) form by escaping Symbols and `$`-prefixed strings:

| Input                              | Marshaled output       |
| ---------------------------------- | ---------------------- |
| `Symbol('foo')` (any sentinel)     | `"$foo"`               |
| A string `"$x"` already starting with `$` | `"$$x"`         |
| Any other string                   | unchanged              |
| `dict`, `list`, `tuple`            | recursed into          |

`JsonDiffer.unmarshal(d)` is the inverse: `"$foo"` decodes back to a Symbol if `foo` is a known label, otherwise it strips a single leading `$`. With `escape_str='$$'` or similar, the prefix changes accordingly.

This means a dict diff like:

```python
{'b': 3, 'c': 4, delete: ['a']}
```

becomes the JSON-encodable:

```python
{'b': 3, 'c': 4, '$delete': ['a']}
```

User data containing literal `$delete` as a string would be marshaled as `"$$delete"`, preserving round-trip safety.

## 5. Worked examples

### 5.1 Dict edits

```python
>>> diff({'a': 1, 'b': 2}, {'b': 3, 'c': 4})
{'b': 3, 'c': 4, delete: ['a']}
```

- `'a'` removed → listed under `delete`.
- `'b'` changed: scalar inequality → diff value is the new scalar `3`.
- `'c'` added → maps directly to its new value `4`.

### 5.2 Nested edits with similarity

```python
>>> diff({'a': [0, {'b': 4}, 1]}, {'a': [0, {'b': 5}, 1]})
{'a': {1: {'b': 5}}}
```

- The list at `'a'` is mostly the same; only the element at target position 1 changed.
- That element is a dict whose only edit is `'b': 4 → 5`, encoded as `{'b': 5}`.

### 5.3 List with insertion, deletion, and change

```python
>>> diff(['x', 'a', {'v': 11}, 'c', 'x'], ['a', {'v': 20}, 'b', 'c'])
{insert: [(2, 'b')], delete: [4, 0], 1: {'v': 20}}
```

Applied to source `['x', 'a', {'v': 11}, 'c', 'x']`:

1. `delete: [4, 0]` → pop position 4 (`'x'`), then position 0 (`'x'`): `['a', {'v': 11}, 'c']`.
2. `insert: [(2, 'b')]` → insert `'b'` at position 2: `['a', {'v': 11}, 'b', 'c']`.
3. Key `1`: patch element at position 1 with `{'v': 20}`, recursing into the dict: `['a', {'v': 20}, 'b', 'c']`.

### 5.4 Sets

```python
>>> diff({1, 2, 4}, {1, 2, 3})
{add: {3}, discard: {4}}
```

### 5.5 Whole-value replacement (mixed types)

```python
>>> diff({'a': 1, 'b': 2}, 123)
123
>>> diff([1, 2], {'c': 3, 'd': 4})
{replace: {'c': 3, 'd': 4}}
```

The second is wrapped in `replace` because the target is a dict and would otherwise be indistinguishable from a dict-patch on the source.

### 5.6 Equal inputs

```python
>>> diff({'a': 1}, {'a': 1})
{}
```

The empty dict means "no change". `patch(a, {}) == a`.

## 6. Invariants

- `patch(a, diff(a, b)) == b` for all JSON-compatible `a`, `b`.
- `diff(a, a) == {}`.
- `marshal` and `unmarshal` are inverses on any diff value.
- The format does **not** support reversal: there is no `unpatch(b, d) → a` operation; the diff retains only the information needed to go forward.
