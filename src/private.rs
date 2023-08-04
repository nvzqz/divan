pub use std::{self, default::Default, option::Option::*};

pub use linkme;

pub use crate::{
    bench::BenchOptions,
    entry::{BenchEntry, EntryLocation, EntryMeta, GroupEntry, BENCH_ENTRIES, GROUP_ENTRIES},
    time::IntoDuration,
};
