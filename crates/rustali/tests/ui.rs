//! Compile-fail tests: `#[controller]` must reject bad input with a clear message.
//!
//! Regenerate expected output with `TRYBUILD=overwrite cargo test -p rustali --test ui`
//! and review the diff.

#[test]
fn controller_errors() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
