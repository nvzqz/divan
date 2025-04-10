use std::{
    mem::ManuallyDrop,
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use regex::Regex;

#[macro_use]
mod macros;

pub mod fmt;
pub mod sort;
pub mod split_vec;
pub mod sync;
pub mod thread;
pub mod ty;

/// Public-in-private type like `()` but meant to be externally-unreachable.
///
/// Using this in place of `()` for `GenI` prevents `Bencher::with_inputs` from
/// working with `()` unintentionally.
#[non_exhaustive]
pub struct Unit;

/// Public-in-private trait to make `DivanConfig::skip_regex` polymorphic over
/// regular expression types.
pub trait IntoRegex {
    fn into_regex(self) -> Regex;
}

impl IntoRegex for Regex {
    #[inline]
    fn into_regex(self) -> Regex {
        self
    }
}

impl IntoRegex for &str {
    #[inline]
    #[track_caller]
    fn into_regex(self) -> Regex {
        Regex::new(self).unwrap()
    }
}

impl IntoRegex for String {
    #[inline]
    #[track_caller]
    fn into_regex(self) -> Regex {
        Regex::new(&self).unwrap()
    }
}

/// [`std::hint::assert_unchecked`] polyfill.
#[inline]
pub(crate) const unsafe fn assert_unchecked(cond: bool) {
    if !cond {
        std::hint::unreachable_unchecked();
    }
}

#[inline]
pub(crate) fn defer<F: FnOnce()>(f: F) -> impl Drop {
    struct Defer<F: FnOnce()>(ManuallyDrop<F>);

    impl<F: FnOnce()> Drop for Defer<F> {
        #[inline]
        fn drop(&mut self) {
            let f = unsafe { ManuallyDrop::take(&mut self.0) };

            f();
        }
    }

    Defer(ManuallyDrop::new(f))
}

/// Returns the index of `ptr` in the slice, assuming it is in the slice.
#[inline]
pub(crate) fn slice_ptr_index<T>(slice: &[T], ptr: *const T) -> usize {
    // Safe pointer `offset_from`.
    (ptr as usize - slice.as_ptr() as usize) / size_of::<T>()
}

/// Returns the values in the middle of `slice`.
///
/// If the slice has an even length, two middle values exist.
#[inline]
pub(crate) fn slice_middle<T>(slice: &[T]) -> &[T] {
    let len = slice.len();

    if len == 0 {
        slice
    } else if len % 2 == 0 {
        &slice[(len / 2) - 1..][..2]
    } else {
        &slice[len / 2..][..1]
    }
}

/// Cached [`std::thread::available_parallelism`].
#[inline]
pub(crate) fn known_parallelism() -> NonZeroUsize {
    static CACHED: AtomicUsize = AtomicUsize::new(0);

    #[cold]
    fn slow() -> NonZeroUsize {
        let n = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);

        match CACHED.compare_exchange(0, n.get(), Relaxed, Relaxed) {
            Ok(_) => n,

            // SAFETY: Zero is checked by us and competing threads.
            Err(n) => unsafe { NonZeroUsize::new_unchecked(n) },
        }
    }

    match NonZeroUsize::new(CACHED.load(Relaxed)) {
        Some(n) => n,
        None => slow(),
    }
}

/// Returns `true` if running under [`cargo-nextest`](https://nexte.st).
pub(crate) fn is_cargo_nextest() -> bool {
    std::env::var_os("NEXTEST").unwrap_or_default() == "1"
}

#[cfg(test)]
mod tests {
    use crate::black_box;

    use super::*;

    #[test]
    fn known_parallelism() {
        let f: fn() -> NonZeroUsize = super::known_parallelism;
        assert_eq!(black_box(f)(), black_box(f)());
    }

    #[test]
    fn slice_middle() {
        use super::slice_middle;

        assert_eq!(slice_middle::<i32>(&[]), &[]);

        assert_eq!(slice_middle(&[1]), &[1]);
        assert_eq!(slice_middle(&[1, 2]), &[1, 2]);
        assert_eq!(slice_middle(&[1, 2, 3]), &[2]);
        assert_eq!(slice_middle(&[1, 2, 3, 4]), &[2, 3]);
        assert_eq!(slice_middle(&[1, 2, 3, 4, 5]), &[3]);
    }
}
