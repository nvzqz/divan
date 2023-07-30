//! Makes Miri more pleasant.

#![allow(unused_variables)]

// https://github.com/rust-lang/miri/blob/master/tests/utils/miri_extern.rs
#[cfg(miri)]
extern "Rust" {
    fn miri_static_root(ptr: *const u8);
}

#[inline]
pub fn leak<T: ?Sized>(ptr: *const T) {
    // SAFETY: Miri will catch invalid pointer usage here, so make this pleasant
    // to use outside of Miri since leaking memory is safe.
    #[cfg(miri)]
    unsafe {
        miri_static_root(ptr.cast());
    }
}
