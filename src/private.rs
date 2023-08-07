pub use std::{self, any, default::Default, option::Option::*};

pub use linkme;

pub use crate::{
    bench::BenchOptions,
    entry::{
        BenchEntry, EntryConst, EntryLocation, EntryMeta, GenericBenchEntry, GroupEntry,
        BENCH_ENTRIES, GROUP_ENTRIES,
    },
    time::IntoDuration,
};

/// Used by `#[divan::bench]` to truncate arrays for generic `const` benchmarks.
pub const fn shrink_array<T, const IN: usize, const OUT: usize>(
    array: [T; IN],
) -> Option<[T; OUT]> {
    use std::mem::ManuallyDrop;

    #[repr(C)]
    union Transmute<F, I> {
        from: ManuallyDrop<F>,
        into: ManuallyDrop<I>,
    }

    let from = ManuallyDrop::new(array);

    if OUT <= IN {
        Some(unsafe { ManuallyDrop::into_inner(Transmute { from }.into) })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shrink_array() {
        let values = [1, 2, 3, 4, 5];

        let equal: Option<[i32; 5]> = super::shrink_array(values);
        assert_eq!(equal, Some(values));

        let smaller: Option<[i32; 3]> = super::shrink_array(values);
        assert_eq!(smaller, Some([1, 2, 3]));

        let larger: Option<[i32; 100]> = super::shrink_array(values);
        assert_eq!(larger, None);
    }
}
