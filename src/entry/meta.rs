use std::sync::OnceLock;

use crate::bench::BenchOptions;

/// Metadata common to `#[divan::bench]` and `#[divan::bench_group]`.
pub struct EntryMeta {
    /// The entry's display name.
    pub display_name: &'static str,

    /// The entry's original name.
    ///
    /// This is used to find a `GroupEntry` for a `BenchEntry`.
    pub raw_name: &'static str,

    /// The entry's raw `module_path!()`.
    pub module_path: &'static str,

    /// Where the entry was defined.
    pub location: EntryLocation,

    /// Configures the benchmarker via attribute options.
    pub get_bench_options: Option<fn() -> BenchOptions<'static>>,

    /// Cached `BenchOptions`.
    pub cached_bench_options: OnceLock<BenchOptions<'static>>,
}

/// Where an entry is located.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub struct EntryLocation {
    pub file: &'static str,
    pub line: u32,
    pub col: u32,
}

impl EntryMeta {
    #[inline]
    pub(crate) fn bench_options(&self) -> Option<&BenchOptions> {
        Some(self.cached_bench_options.get_or_init(self.get_bench_options?))
    }

    #[inline]
    pub(crate) fn module_path_components<'a>(&self) -> impl Iterator<Item = &'a str> {
        self.module_path.split("::")
    }
}
