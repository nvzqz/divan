//! Synchronization utilities.

#![cfg(target_os = "macos")]

use std::{
    marker::PhantomData,
    sync::atomic::{Ordering::*, *},
};

use libc::pthread_key_t;

const KEY_UNINIT: pthread_key_t = 0;

/// Thread-local key accessed via
/// [`pthread_getspecific`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_getspecific.html).
pub(crate) struct PThreadKey<T: 'static> {
    value: AtomicPThreadKey,
    marker: PhantomData<&'static T>,
}

impl<T> PThreadKey<T> {
    #[inline]
    pub const fn new() -> Self {
        Self { value: AtomicPThreadKey::new(KEY_UNINIT), marker: PhantomData }
    }

    #[inline]
    pub fn get(&self) -> Option<&'static T> {
        match self.value.load(Relaxed) {
            KEY_UNINIT => None,
            key => unsafe { libc::pthread_getspecific(key).cast::<T>().as_ref() },
        }
    }

    /// Assigns the value with its destructor.
    #[inline]
    pub fn set<D>(&self, value: &'static T, _: D)
    where
        D: FnOnce(&'static T) + Copy,
    {
        assert_eq!(std::mem::size_of::<D>(), 0);

        unsafe extern "C" fn dtor<T, D>(value: *mut libc::c_void)
        where
            T: 'static,
            D: FnOnce(&'static T) + Copy,
        {
            // SAFETY: The dtor is zero-sized, so we can make one from thin air.
            let dtor: D = unsafe { std::mem::zeroed() };

            dtor(unsafe { &*value.cast() });
        }

        let shared_key = &self.value;
        let mut local_key = shared_key.load(Relaxed);

        // Race against other threads to initialize `shared_key`.
        if local_key == KEY_UNINIT {
            if unsafe { libc::pthread_key_create(&mut local_key, Some(dtor::<T, D>)) } == 0 {
                // Race to store our key into the global instance.
                //
                // On failure, delete our key and use the winner's key.
                if let Err(their_key) =
                    shared_key.compare_exchange(KEY_UNINIT, local_key, Relaxed, Relaxed)
                {
                    // SAFETY: No other thread is accessing this key.
                    unsafe { libc::pthread_key_delete(local_key) };

                    local_key = their_key;
                }
            } else {
                // On create failure, check if another thread succeeded.
                local_key = shared_key.load(Relaxed);
                if local_key == KEY_UNINIT {
                    return;
                }
            }
        }

        // SAFETY: The key has been created by us or another thread.
        unsafe { libc::pthread_setspecific(local_key, value as *const T as _) };
    }
}

/// Alias to the atomic equivalent of `pthread_key_t`.
pub(crate) type AtomicPThreadKey = Atomic<pthread_key_t>;

/// Alias to the atomic equivalent of `T`.
pub(crate) type Atomic<T> = <T as WithAtomic>::Atomic;

/// A type with an associated atomic type.
pub(crate) trait WithAtomic {
    type Atomic;
}

#[cfg(target_has_atomic = "ptr")]
impl WithAtomic for usize {
    type Atomic = AtomicUsize;
}

#[cfg(target_has_atomic = "ptr")]
impl WithAtomic for isize {
    type Atomic = AtomicIsize;
}

#[cfg(target_has_atomic = "8")]
impl WithAtomic for u8 {
    type Atomic = AtomicU8;
}

#[cfg(target_has_atomic = "8")]
impl WithAtomic for i8 {
    type Atomic = AtomicI8;
}

#[cfg(target_has_atomic = "16")]
impl WithAtomic for u16 {
    type Atomic = AtomicU16;
}

#[cfg(target_has_atomic = "16")]
impl WithAtomic for i16 {
    type Atomic = AtomicI16;
}

#[cfg(target_has_atomic = "32")]
impl WithAtomic for u32 {
    type Atomic = AtomicU32;
}

#[cfg(target_has_atomic = "32")]
impl WithAtomic for i32 {
    type Atomic = AtomicI32;
}

#[cfg(target_has_atomic = "64")]
impl WithAtomic for u64 {
    type Atomic = AtomicU64;
}

#[cfg(target_has_atomic = "64")]
impl WithAtomic for i64 {
    type Atomic = AtomicI64;
}
