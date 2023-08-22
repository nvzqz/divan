use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
    thread::{Thread, ThreadId},
};

fn main() {
    divan::main();
}

/// Benchmark getting an integer or pointer uniquely identifying the current
/// thread or core.
mod thread_id {
    use super::*;

    #[divan::bench_group(name = "std")]
    mod stdlib {
        use super::*;

        mod thread_local {
            use super::*;

            #[divan::bench]
            fn count() -> usize {
                static SHARED: AtomicUsize = AtomicUsize::new(0);

                thread_local! {
                    static LOCAL: usize = SHARED.fetch_add(1, Relaxed);
                }

                LOCAL.with(|count| *count)
            }

            #[divan::bench]
            fn id() -> ThreadId {
                thread_local! {
                    static LOCAL: ThreadId = std::thread::current().id();
                }

                LOCAL.with(|id| *id)
            }

            #[divan::bench]
            fn ptr() -> *mut u8 {
                thread_local! {
                    static LOCAL: UnsafeCell<u8> = UnsafeCell::new(0);
                }

                LOCAL.with(|addr| addr.get())
            }
        }

        mod thread {
            use super::*;

            #[divan::bench]
            fn current() -> Thread {
                std::thread::current()
            }

            #[divan::bench]
            fn current_id() -> ThreadId {
                std::thread::current().id()
            }
        }
    }

    mod pthread {
        // https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_self.html
        #[cfg(unix)]
        #[divan::bench(name = "self")]
        fn this() -> libc::pthread_t {
            unsafe { libc::pthread_self() }
        }

        #[cfg(target_os = "macos")]
        #[divan::bench]
        fn get_stackaddr_np() -> *mut libc::c_void {
            unsafe { libc::pthread_get_stackaddr_np(libc::pthread_self()) }
        }

        #[cfg(target_os = "macos")]
        #[divan::bench]
        fn threadid_np() -> u64 {
            unsafe {
                let mut tid = 0;
                libc::pthread_threadid_np(libc::pthread_self(), &mut tid);
                tid
            }
        }
    }

    // https://www.gnu.org/software/hurd/gnumach-doc/Thread-Information.html
    #[cfg(target_os = "macos")]
    #[divan::bench]
    fn mach_thread_self() -> libc::thread_t {
        unsafe { libc::mach_thread_self() }
    }

    // https://man7.org/linux/man-pages/man2/gettid.2.html
    #[cfg(target_os = "linux")]
    #[divan::bench]
    fn gettid() -> libc::pid_t {
        unsafe { libc::gettid() }
    }

    // https://man7.org/linux/man-pages/man3/sched_getcpu.3.html
    #[cfg(target_os = "linux")]
    #[divan::bench]
    fn sched_getcpu() -> libc::c_int {
        unsafe { libc::sched_getcpu() }
    }

    #[cfg(windows)]
    #[divan::bench]
    #[allow(non_snake_case)]
    fn GetCurrentThreadId() -> u32 {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThreadId() -> u32;
        }

        unsafe { GetCurrentThreadId() }
    }

    // https://developer.arm.com/documentation/ddi0595/2021-12/AArch64-Registers/TPIDRRO-EL0--EL0-Read-Only-Software-Thread-ID-Register?lang=en
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    #[divan::bench]
    fn cpu() -> usize {
        unsafe {
            let result: usize;
            std::arch::asm!(
                "mrs {}, tpidrro_el0",
                out(reg) result,
                options(nostack, nomem, preserves_flags)
            );
            result
        }
    }

    // https://developer.arm.com/documentation/ddi0595/2021-12/AArch64-Registers/TPIDR-EL0--EL0-Read-Write-Software-Thread-ID-Register?lang=en
    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    #[divan::bench]
    fn cpu() -> usize {
        unsafe {
            let result: usize;
            std::arch::asm!(
                "mrs {}, tpidr_el0",
                out(reg) result,
                options(nostack, nomem, preserves_flags)
            );
            result
        }
    }
}
