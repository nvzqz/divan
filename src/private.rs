use std::borrow::Borrow;
pub use std::{
    self, any, borrow::Cow, default::Default, iter::FromIterator, option::Option::*, sync::OnceLock,
};

pub use crate::{
    bench::BenchOptions,
    entry::{
        BenchEntry, EntryConst, EntryList, EntryLocation, EntryMeta, EntryType, GenericBenchEntry,
        GroupEntry, BENCH_ENTRIES, GROUP_ENTRIES,
    },
    time::IntoDuration,
};

/// Used by `#[divan::bench(threads = ...)]` to leak thread counts for easy
/// global usage in [`BenchOptions::threads`].
///
/// This enables the `threads` option to be polymorphic over:
/// - `usize`
/// - `bool`
///     - `true` is 0
///     - `false` is 1
/// - Iterators:
///     - `[usize; N]`
///     - `&[usize; N]`
///     - `&[usize]`
///
/// # Orphan Rules Hack
///
/// Normally we can't implement a trait over both `usize` and `I: IntoIterator`
/// because the compiler has no guarantee that `usize` will never implement
/// `IntoIterator`. Ideally we would handle this with specialization, but that's
/// not stable.
///
/// The solution here is to make `IntoThreads` generic to implement technically
/// different traits for `usize` and `IntoIterator` because of different `IMP`
/// values. We then call verbatim `IntoThreads::into_threads(val)` and have the
/// compiler infer the generic parameter for the single `IntoThreads`
/// implementation.
///
/// It's fair to assume that scalar primitives will never implement
/// `IntoIterator`, so this hack shouldn't break in the future 🤠.
pub trait IntoThreads<const IMP: u32> {
    fn into_threads(self) -> Cow<'static, [usize]>;
}

impl IntoThreads<0> for usize {
    #[inline]
    fn into_threads(self) -> Cow<'static, [usize]> {
        let counts = match self {
            0 => &[0],
            1 => &[1],
            2 => &[2],
            _ => return Cow::Owned(vec![self]),
        };
        Cow::Borrowed(counts)
    }
}

impl IntoThreads<0> for bool {
    #[inline]
    fn into_threads(self) -> Cow<'static, [usize]> {
        let counts = if self {
            // Available parallelism.
            &[0]
        } else {
            // No parallelism.
            &[1]
        };
        Cow::Borrowed(counts)
    }
}

impl<I> IntoThreads<1> for I
where
    I: IntoIterator,
    I::Item: Borrow<usize>,
{
    #[inline]
    fn into_threads(self) -> Cow<'static, [usize]> {
        let mut options: Vec<usize> = self.into_iter().map(|i| *i.borrow()).collect();
        options.sort_unstable();
        options.dedup();
        Cow::Owned(options)
    }
}

/// Used by `#[divan::bench(counters = [...])]`.
#[inline]
pub fn new_counter_set() -> crate::counter::CounterSet {
    Default::default()
}

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
    use super::*;

    #[test]
    fn into_threads() {
        macro_rules! test {
            ($value:expr, $expected:expr) => {
                assert_eq!(IntoThreads::into_threads($value).as_ref(), $expected);
            };
        }

        test!(true, &[0]);
        test!(false, &[1]);

        test!(0, &[0]);
        test!(1, &[1]);
        test!(42, &[42]);

        test!([0; 0], &[]);
        test!([0], &[0]);
        test!([0, 0], &[0]);

        test!([0, 2, 3, 1], &[0, 1, 2, 3]);
        test!([0, 0, 2, 3, 2, 1, 3], &[0, 1, 2, 3]);
    }

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
