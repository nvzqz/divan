//! Synchronization utilities.

#![cfg(target_os = "macos")]

use std::{
    marker::PhantomData,
    sync::atomic::{Ordering::*, *},
};

use cfg_if::cfg_if;
use libc::pthread_key_t;

const KEY_UNINIT: pthread_key_t = 0;

/// Returns a pointer to the corresponding thread-local variable.
///
/// The first element is reserved for `pthread_self`. This is widely known and
/// also mentioned in page 251 of "*OS Internals Volume 1" by Jonathan Levin.
///
/// It appears that `pthread_key_create` allocates a slot into the buffer
/// referenced by `gs` on x86_64 and `tpidrro_el0` on AArch64.
///
/// # Safety
///
/// `key` must not cause an out-of-bounds lookup.
#[inline]
#[cfg(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(miri)))]
unsafe fn get_thread_local(key: usize) -> *mut libc::c_void {
    #[cfg(target_arch = "x86_64")]
    {
        let result;
        std::arch::asm!(
            // https://github.com/apple-oss-distributions/xnu/blob/xnu-10002.41.9/libsyscall/os/tsd.h#L126
            "mov {0}, gs:[8 * {1}]",
            out(reg) result,
            in(reg) key,
            options(pure, readonly, nostack, preserves_flags),
        );
        result
    }

    #[cfg(target_arch = "aarch64")]
    {
        let result: *const *mut libc::c_void;
        std::arch::asm!(
            // https://github.com/apple-oss-distributions/xnu/blob/xnu-10002.41.9/libsyscall/os/tsd.h#L163
            "mrs {0}, tpidrro_el0",
            // Clear bottom 3 bits just in case. This was historically the CPU
            // core ID but that changed at some point.
            "and {0}, {0}, #-8",
            out(reg) result,
            options(pure, nomem, nostack, preserves_flags),
        );
        *result.add(key)
    }
}

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

            key => unsafe {
                #[allow(clippy::needless_late_init)]
                let thread_local: *mut libc::c_void;

                cfg_if! {
                    if #[cfg(all(
                        any(target_arch = "x86_64", target_arch = "aarch64"),
                        not(miri)
                    ))] {
                        thread_local = get_thread_local(key as usize);

                        #[cfg(test)]
                        assert_eq!(thread_local, libc::pthread_getspecific(key));
                    } else {
                        thread_local = libc::pthread_getspecific(key);
                    }
                }

                thread_local.cast::<T>().as_ref()
            },
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

        // This is the slow path, so don't bother with writing via
        // `gs`/`tpidrro_el0` register.
        //
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
