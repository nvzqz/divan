// Tests that entry benchmarks/groups have correct generated properties.

// Miri does not work with `linkme`.
#![cfg(not(miri))]

use divan::__private::{Entry, EntryGroup, ENTRIES, ENTRY_GROUPS};

#[divan::bench]
fn outer() {}

#[divan::bench_group]
mod outer_group {
    #[divan::bench]
    fn inner() {}

    #[divan::bench_group]
    mod inner_group {}
}

#[divan::bench]
#[ignore]
fn ignored() {}

#[divan::bench_group]
#[allow(unused_attributes)]
#[ignore]
mod ignored_group {
    #[divan::bench]
    fn not_yet_ignored() {}
}

/// Finds an `Entry` or `EntryGroup` based on its raw name.
macro_rules! find {
    ($entries:expr, $raw_name:literal) => {
        $entries
            .iter()
            .find(|entry| entry.raw_name == $raw_name)
            .expect(concat!($raw_name, " not found"))
    };
}

fn find_outer() -> &'static Entry {
    find!(ENTRIES, "outer")
}

fn find_inner() -> &'static Entry {
    find!(ENTRIES, "inner")
}

fn find_outer_group() -> &'static EntryGroup {
    find!(ENTRY_GROUPS, "outer_group")
}

fn find_inner_group() -> &'static EntryGroup {
    find!(ENTRY_GROUPS, "inner_group")
}

#[test]
fn file() {
    let file = file!();

    assert_eq!(find_outer().file, file);
    assert_eq!(find_outer_group().file, file);

    assert_eq!(find_inner().file, file);
    assert_eq!(find_inner_group().file, file);
}

#[test]
fn module_path() {
    let outer_path = module_path!();
    assert_eq!(find_outer().module_path, outer_path);
    assert_eq!(find_outer_group().module_path, outer_path);

    let inner_path = format!("{outer_path}::outer_group");
    assert_eq!(find_inner().module_path, inner_path);
    assert_eq!(find_inner_group().module_path, inner_path);
}

#[test]
fn line() {
    assert_eq!(find_outer().line, 8);
    assert_eq!(find_outer_group().line, 11);

    assert_eq!(find_inner().line, 13);
    assert_eq!(find_inner_group().line, 16);
}

#[test]
fn column() {
    assert_eq!(find_outer().col, 1);
    assert_eq!(find_outer_group().col, 1);

    assert_eq!(find_inner().col, 5);
    assert_eq!(find_inner_group().col, 5);
}

#[test]
fn ignore() {
    assert!(find!(ENTRIES, "ignored").ignore);
    assert!(find!(ENTRY_GROUPS, "ignored_group").ignore);

    // Although its parent is marked as `#[ignore]`, it itself is not yet known
    // to be ignored.
    assert!(!find!(ENTRIES, "not_yet_ignored").ignore);

    assert!(!find_inner().ignore);
    assert!(!find_inner_group().ignore);
    assert!(!find_outer().ignore);
    assert!(!find_outer_group().ignore);
}
