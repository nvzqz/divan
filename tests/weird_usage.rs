// Tests that ensure weird (but valid) usage behave as expected.

use std::{
    any::TypeId,
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering as AtomicOrdering},
};

use divan::{
    Divan,
    __private::{Entry, ENTRIES},
};

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

static TWICE_RUNS: AtomicUsize = AtomicUsize::new(0);
static THRICE_RUNS: AtomicUsize = AtomicUsize::new(0);

#[divan::bench]
#[divan::bench]
fn twice() {
    TWICE_RUNS.fetch_add(1, AtomicOrdering::Relaxed);
}

#[divan::bench]
#[divan::bench]
#[divan::bench]
fn thrice() {
    THRICE_RUNS.fetch_add(1, AtomicOrdering::Relaxed);
}

#[test]
fn test_fn() {
    Divan::default().test();

    // `Divan::test` should deduplicate benchmark registration.
    assert_eq!(TWICE_RUNS.load(AtomicOrdering::Relaxed), 1);
    assert_eq!(THRICE_RUNS.load(AtomicOrdering::Relaxed), 1);
}

// Test that each function appears the expected number of times.
#[test]
fn count() {
    let mut inner_count = 0;
    let mut twice_count = 0;
    let mut thrice_count = 0;

    for entry in ENTRIES {
        if entry.path.contains("inner") {
            inner_count += 1;
        }
        if entry.path.contains("twice") {
            twice_count += 1;
        }
        if entry.path.contains("thrice") {
            thrice_count += 1;
        }
    }

    assert_eq!(inner_count, 2);
    assert_eq!(twice_count, 2);
    assert_eq!(thrice_count, 3);
}

// Test expected `Entry.path` values.
#[test]
fn path() {
    for entry in ENTRIES {
        // Embedded functions do not contain their parent function's name in
        // their path because that's not how `module_path!()` works.
        if entry.path.contains("inner") {
            assert_eq!(entry.path, "weird_usage::inner");
        }

        // "r#" is removed from raw identifiers.
        if entry.path.contains("raw_ident") {
            assert_eq!(entry.path, "weird_usage::raw_ident");
        }
    }
}

// Test that each benchmarked function has a unique type ID.
#[test]
fn unique_id() {
    let mut seen = HashMap::<TypeId, &Entry>::new();

    for entry in ENTRIES {
        if entry.path.contains("twice") || entry.path.contains("thrice") {
            continue;
        }

        let id = (entry.get_id)();

        if let Some(collision) = seen.insert(id, entry) {
            fn info(entry: &Entry) -> String {
                format!("'{}' ({}:{})", entry.path, entry.file, entry.line)
            }

            panic!("Type ID collision for {} and {}", info(collision), info(entry));
        }
    }
}
