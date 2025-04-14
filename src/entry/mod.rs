use std::ptr::NonNull;

use crate::{benchmark::BenchArgsRunner, Bencher};

mod generic;
mod list;
mod meta;
mod tree;

pub use self::{
    generic::{EntryConst, EntryType, GenericBenchEntry},
    list::EntryList,
    meta::{EntryLocation, EntryMeta},
};
pub(crate) use tree::EntryTree;

/// Benchmark entries generated by `#[divan::bench]`.
///
/// Note: generic-type benchmark entries are instead stored in `GROUP_ENTRIES`
/// in `generic_benches`.
pub static BENCH_ENTRIES: EntryList<BenchEntry> = EntryList::root();

/// Group entries generated by `#[divan::bench_group]`.
pub static GROUP_ENTRIES: EntryList<GroupEntry> = EntryList::root();

/// Determines how the benchmark entry is run.
#[derive(Clone, Copy)]
pub enum BenchEntryRunner {
    /// Benchmark without arguments.
    Plain(fn(Bencher)),

    /// Benchmark with runtime arguments.
    Args(fn() -> BenchArgsRunner),
}

/// Compile-time entry for a benchmark, generated by `#[divan::bench]`.
pub struct BenchEntry {
    /// Entry metadata.
    pub meta: EntryMeta,

    /// The benchmarking function.
    pub bench: BenchEntryRunner,
}

/// Compile-time entry for a benchmark group, generated by
/// `#[divan::bench_group]` or a generic-type `#[divan::bench]`.
pub struct GroupEntry {
    /// Entry metadata.
    pub meta: EntryMeta,

    /// Generic `#[divan::bench]` entries.
    ///
    /// This is two-dimensional to make code generation simpler. The outer
    /// dimension corresponds to types and the inner dimension corresponds to
    /// constants.
    pub generic_benches: Option<&'static [&'static [GenericBenchEntry]]>,
}

impl GroupEntry {
    pub(crate) fn generic_benches_iter(
        &self,
    ) -> impl Iterator<Item = &'static GenericBenchEntry> {
        self.generic_benches
            .unwrap_or_default()
            .iter()
            .flat_map(|benches| benches.iter())
    }
}

/// `BenchEntry` or `GenericBenchEntry`.
#[derive(Clone, Copy)]
pub(crate) enum AnyBenchEntry<'a> {
    Bench(&'a BenchEntry),
    GenericBench(&'a GenericBenchEntry),
}

impl<'a> AnyBenchEntry<'a> {
    /// Returns a pointer to use as the identity of the entry.
    #[inline]
    pub fn entry_addr(self) -> NonNull<()> {
        match self {
            Self::Bench(entry) => NonNull::from(entry).cast(),
            Self::GenericBench(entry) => NonNull::from(entry).cast(),
        }
    }

    /// Returns this entry's benchmark runner.
    #[inline]
    pub fn bench_runner(self) -> &'a BenchEntryRunner {
        match self {
            Self::Bench(BenchEntry { bench, .. })
            | Self::GenericBench(GenericBenchEntry { bench, .. }) => bench,
        }
    }

    /// Returns this entry's argument names.
    #[inline]
    pub fn arg_names(self) -> Option<&'static [&'static str]> {
        match self.bench_runner() {
            BenchEntryRunner::Args(bench_runner) => {
                let bench_runner = bench_runner();
                Some(bench_runner.arg_names())
            }
            _ => None,
        }
    }

    #[inline]
    pub fn meta(self) -> &'a EntryMeta {
        match self {
            Self::Bench(entry) => &entry.meta,
            Self::GenericBench(entry) => &entry.group.meta,
        }
    }

    #[inline]
    pub fn raw_name(self) -> &'a str {
        match self {
            Self::Bench(entry) => entry.meta.raw_name,
            Self::GenericBench(entry) => entry.raw_name(),
        }
    }

    #[inline]
    pub fn display_name(self) -> &'a str {
        match self {
            Self::Bench(entry) => entry.meta.display_name,
            Self::GenericBench(entry) => entry.display_name(),
        }
    }
}
