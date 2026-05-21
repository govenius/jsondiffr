//! Placeholder for the differ test suite.
//!
//! The Rust differ is not yet implemented (see `src/lib.rs`). When it lands,
//! this file should grow the test functions described below. They mirror
//! the deferred portion of `tests/test_jsondiff.py` — Python tests whose
//! assertions go through `diff()` and therefore cannot be ported today.
//!
//! # Deferred until the Rust differ exists
//!
//! The following Python tests *cannot* be ported without `diff()`. Re-plan
//! when the differ lands:
//!
//! * All of `JsonDiffTests::test_a` (currently ~30 `diff(...)` equality cases).
//! * `test_marshal` — round-trip needs both `marshal()` and `unmarshal()`.
//! * `test_dump`, `test_compact_syntax` — hypothesis-driven property tests;
//!   would port to `proptest` once a differ is callable. Use the round-trip
//!   property `patch(a, diff(a, b)) == b`.
//! * The §3 *diff() behavior* group from `TestFormatConformance`:
//!   `diff_low_similarity_emits_bare_or_wrapped_b`,
//!   `diff_list_delete_positions_descending`,
//!   `diff_unchanged_subkeys_omitted`,
//!   `diff_against_empty_uses_replace_wrap`.
//! * The §4 invariants `diff_identity` and the cross-cutting
//!   `diff_does_not_mutate_inputs`.
