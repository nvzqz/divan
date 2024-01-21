use std::{
    any::{Any, TypeId},
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

pub mod fmt;
pub mod sync;

/// Public-in-private type like `()` but meant to be externally-unreachable.
///
/// Using this in place of `()` for `GenI` prevents `Bencher::with_inputs` from
/// working with `()` unintentionally.
#[non_exhaustive]
pub struct Unit;

/// Makes the wrapped value [`Send`] + [`Sync`] even though it isn't.
pub struct SyncWrap<T> {
    value: T,
}

unsafe impl<T> Sync for SyncWrap<T> {}

impl<T> Deref for SyncWrap<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for SyncWrap<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> SyncWrap<T> {
    #[inline]
    pub const unsafe fn new(value: T) -> Self {
        Self { value }
    }
}

#[inline]
pub(crate) fn cast_ref<T: Any>(r: &impl Any) -> Option<&T> {
    if r.type_id() == TypeId::of::<T>() {
        // SAFETY: `r` is `&T`.
        Some(unsafe { &*(r as *const _ as *const T) })
    } else {
        None
    }
}

/// Returns the index of `ptr` in the slice, assuming it is in the slice.
#[inline]
pub(crate) fn slice_ptr_index<T>(slice: &[T], ptr: *const T) -> usize {
    // Safe pointer `offset_from`.
    (ptr as usize - slice.as_ptr() as usize) / std::mem::size_of::<T>()
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
