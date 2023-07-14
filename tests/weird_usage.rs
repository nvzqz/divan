// Tests that ensure weird (but valid) usage behave as expected.

// Miri does not work with `linkme`.
#![cfg(not(miri))]

use divan::{Divan, __private::ENTRIES};

#[divan::bench]
fn lifetime<'a>() -> &'a str {
    "hello"
}

#[divan::bench]
fn embedded() {
    #[divan::bench]
    fn inner() {
        #[divan::bench]
        fn inner() {}
    }
}

#[divan::bench]
fn r#raw_ident() {}

#[test]
fn test_fn() {
    Divan::default().test();
}

// Test that each function appears the expected number of times.
#[test]
fn count() {
    let mut inner_count = 0;

    for entry in ENTRIES {
        if entry.raw_name == "inner" {
            inner_count += 1;
        }
    }

    assert_eq!(inner_count, 2);
}

// Test expected `Entry.path` values.
#[test]
fn path() {
    for entry in ENTRIES {
        // Embedded functions do not contain their parent function's name in
        // their `module_path!()`.
        if entry.raw_name == "inner" {
            assert_eq!(entry.module_path, "weird_usage");
        }

        // "r#" is removed from raw identifiers.
        if entry.raw_name.contains("raw_ident") {
            assert_eq!(entry.raw_name, "r#raw_ident");
            assert_eq!(entry.display_name, "raw_ident");
        }
    }
}
