//! Compile-fail tests: `#[derive(Entity)]` must reject bad input with a clear message.
//!
//! Expected output lives next to each case as `<case>.stderr`. Regenerate with
//! `TRYBUILD=overwrite cargo test -p rustali-orm --test ui` and review the diff.

#[test]
fn derive_entity_errors() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
