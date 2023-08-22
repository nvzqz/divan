//! Makes Miri more pleasant.

#![allow(unused_variables)]

// https://github.com/rust-lang/miri/blob/master/tests/utils/miri_extern.rs
#[cfg(miri)]
extern "Rust" {
    fn miri_static_root(ptr: *const u8);
}

/// Make Miri quiet about leaking `val`.
#[inline]
pub fn leak<T: ?Sized>(val: &'static T) -> &T {
    #[cfg(miri)]
    unsafe {
        if std::mem::size_of_val(val) != 0 {
            miri_static_root(val as *const _ as _);
        }
    }
    val
}
