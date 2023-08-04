pub use std::{self, any, default::Default, option::Option::*};

pub use linkme;

pub use crate::{
    bench::BenchOptions,
    entry::{
        BenchEntry, EntryLocation, EntryMeta, GenericBenchEntry, GroupEntry, BENCH_ENTRIES,
        GROUP_ENTRIES,
    },
    time::IntoDuration,
};
