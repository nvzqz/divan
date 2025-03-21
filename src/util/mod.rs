use std::{
    mem::ManuallyDrop,
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

pub mod fmt;
pub mod sort;
pub mod sync;
pub mod thread;
pub mod ty;

/// Public-in-private type like `()` but meant to be externally-unreachable.
///
/// Using this in place of `()` for `GenI` prevents `Bencher::with_inputs` from
/// working with `()` unintentionally.
#[non_exhaustive]
pub struct Unit;

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

pub(crate) trait Sqrt {
    fn sqrt(self) -> Self;
}

impl Sqrt for f64 {
    fn sqrt(self) -> Self {
        self.sqrt()
    }
}

// The reason for this algorithm here instead of
// simply casting the integer to f64 and using sqrt
// is the loss of precision.
//
// Consider that the stddev is 100s, which is
// 10^14 ps. Its squared value is 10^28 ps^2, or
// approximately 2^93 ps^2. The f64's mantissa is
// 53 bits, which means, adding some 2^(93 - 53) =
// 2^40 has no impact on the value of f64. That means
// the loss of precision while computing standard
// deviation will be sqrt(2^40), which is around
// 1'000'000 ps ≈ 1 µs. This isn't much for a 100s
// standard deviation, but still can be avoided
// easily.
//
// See test f64_prec below.
impl Sqrt for u128 {
    fn sqrt(self) -> Self {
        if self <= 1 {
            return self;
        }
        let mut x0 = self / 2;
        let mut x1 = (x0 + self / x0) / 2;
        while x1 < x0 {
            x0 = x1;
            x1 = (x0 + self / x0) / 2;
        }
        x0
    }
}

/// Standard deviation on a sequence generated
/// by the iterator.
pub(crate) fn stddev<T, IT>(it: IT, len: T, mean: T) -> T
where
    IT: Iterator<Item = T>,
    T: std::ops::Mul<Output = T>
        + Sqrt
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Copy
        + std::ops::Div<Output = T>,
    T: std::iter::Sum<IT::Item>,
{
    let variance: T = it
        .map(|x| {
            let diff = if x > mean { x - mean } else { mean - x };
            diff * diff
        })
        .sum::<T>()
        / len;
    variance.sqrt()
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

    #[test]
    fn stddev() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let expected = 2.0f64.sqrt();
        let actual = crate::util::stddev(data.iter().copied(), data.len() as f64, mean);
        assert!((expected - actual).abs() < f64::EPSILON);
    }

    #[test]
    fn f64_prec() {
        const EXP: u32 = 93;
        const F64_MANTISSA: u32 = EXP - 53;
        let a1 = 2u128.pow(EXP);
        let a2 = a1 - 1 + 2u128.pow(F64_MANTISSA - 1);
        let f1 = a1 as f64;
        let f2 = a2 as f64;
        assert_eq!(f1, f2);
    }

    #[test]
    fn u128_sqrt() {
        assert_eq!(0u128.sqrt(), 0);
        assert_eq!(1u128.sqrt(), 1);
        assert_eq!(101u128.sqrt(), 10);
        assert_eq!(102390123.sqrt(), 10118);
    }
}
