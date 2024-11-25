//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench threads
//! ```

use std::{
    cell::UnsafeCell,
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        Arc, Mutex, RwLock,
    },
    thread::{Thread, ThreadId},
};

use divan::{black_box, black_box_drop, AllocProfiler, Bencher};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// Available parallelism (0), baseline (1), and common CPU core counts.
const THREADS: &[usize] = &[0, 1, 4, 16];

#[divan::bench_group(threads = THREADS)]
mod arc {
    use super::*;

    #[divan::bench]
    fn clone(bencher: Bencher) {
        let arc = Arc::new(42);
        bencher.bench(|| arc.clone());
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        let arc = Arc::new(42);
        bencher.with_inputs(|| arc.clone()).bench_values(std::mem::drop);
    }

    #[divan::bench]
    fn get_mut(bencher: Bencher) {
        let arc = Arc::new(42);

        bencher.with_inputs(|| arc.clone()).bench_refs(|arc| {
            // Black box the branched value to ensure a branch gets emitted.
            // This more closely simulates `Arc::get_mut` usage in practice.
            if let Some(val) = Arc::get_mut(arc) {
                black_box_drop(val);
            }
        });
    }
}

#[divan::bench_group(threads = THREADS)]
mod mutex {
    use super::*;

    mod lock {
        use super::*;

        #[divan::bench]
        fn block() {
            static M: Mutex<u64> = Mutex::new(0);
            black_box_drop(M.lock());
        }

        #[divan::bench]
        fn r#try() {
            static M: Mutex<u64> = Mutex::new(0);
            black_box_drop(M.try_lock());
        }
    }

    mod set {
        use super::*;

        #[divan::bench]
        fn block() {
            static M: Mutex<u64> = Mutex::new(0);
            *black_box(M.lock().unwrap()) = black_box(42);
        }

        #[divan::bench]
        fn r#try() {
            static M: Mutex<u64> = Mutex::new(0);

            if let Ok(lock) = M.try_lock() {
                *black_box(lock) = black_box(42);
            }
        }
    }
}

#[divan::bench_group(threads = THREADS)]
mod rw_lock {
    use super::*;

    mod read {
        use super::*;

        #[divan::bench]
        fn block() {
            static L: RwLock<u64> = RwLock::new(0);
            black_box_drop(L.read());
        }

        #[divan::bench]
        fn r#try() {
            static L: RwLock<u64> = RwLock::new(0);
            black_box_drop(L.try_read());
        }
    }

    mod write {
        use super::*;

        #[divan::bench]
        fn block() {
            static L: RwLock<u64> = RwLock::new(0);
            black_box_drop(L.write());
        }

        #[divan::bench]
        fn r#try() {
            static L: RwLock<u64> = RwLock::new(0);
            black_box_drop(L.try_write());
        }
    }

    mod set {
        use super::*;

        #[divan::bench]
        fn block() {
            static L: RwLock<u64> = RwLock::new(0);
            *black_box(L.write().unwrap()) = black_box(42);
        }

        #[divan::bench]
        fn r#try() {
            static L: RwLock<u64> = RwLock::new(0);

            if let Ok(lock) = L.try_write() {
                *black_box(lock) = black_box(42);
            }
        }
    }
}

/// Benchmark getting an integer or pointer uniquely identifying the current
/// thread or core.
#[divan::bench_group(threads = THREADS)]
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
                    static LOCAL: UnsafeCell<u8> = const { UnsafeCell::new(0) };
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

    #[cfg(unix)]
    mod pthread {
        use super::*;

        // https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_self.html
        #[divan::bench(name = "self")]
        fn this() -> libc::pthread_t {
            unsafe { libc::pthread_self() }
        }

        #[divan::bench]
        fn getspecific(bencher: Bencher) {
            unsafe {
                let mut key: libc::pthread_key_t = 0;
                loop {
                    match libc::pthread_key_create(&mut key, None) {
                        0 => break,
                        libc::EAGAIN => continue,
                        error => panic!("{}", std::io::Error::from_raw_os_error(error)),
                    }
                }

                bencher.bench(|| libc::pthread_getspecific(key));

                libc::pthread_key_delete(key);
            };
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

        #[cfg(target_os = "macos")]
        #[divan::bench]
        fn cpu_number_np() -> usize {
            unsafe {
                let mut cpu = 0;
                libc::pthread_cpu_number_np(&mut cpu);
                cpu
            }
        }
    }

    // https://www.gnu.org/software/hurd/gnumach-doc/Thread-Information.html
    #[cfg(target_os = "macos")]
    #[divan::bench]
    fn mach_thread_self() -> impl Drop {
        struct Thread(mach2::mach_types::thread_port_t);

        impl Drop for Thread {
            fn drop(&mut self) {
                unsafe {
                    mach2::mach_port::mach_port_deallocate(mach2::traps::mach_task_self(), self.0);
                }
            }
        }

        Thread(unsafe { mach2::mach_init::mach_thread_self() })
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
    fn GetCurrentProcessorNumber() -> u32 {
        unsafe { winapi::um::processthreadsapi::GetCurrentProcessorNumber() }
    }

    #[cfg(windows)]
    #[divan::bench]
    #[allow(non_snake_case)]
    fn GetCurrentProcessorNumberEx() -> (u16, u8) {
        unsafe {
            let mut result = std::mem::zeroed();
            winapi::um::processthreadsapi::GetCurrentProcessorNumberEx(&mut result);
            (result.Group, result.Number)
        }
    }

    #[cfg(windows)]
    #[divan::bench]
    #[allow(non_snake_case)]
    fn GetCurrentThread() -> std::os::windows::io::RawHandle {
        unsafe { winapi::um::processthreadsapi::GetCurrentThread().cast() }
    }

    #[cfg(windows)]
    #[divan::bench]
    #[allow(non_snake_case)]
    fn GetCurrentThreadId() -> u32 {
        unsafe { winapi::um::processthreadsapi::GetCurrentThreadId() }
    }

    #[cfg(windows)]
    #[divan::bench]
    #[allow(non_snake_case)]
    fn TlsGetValue(bencher: Bencher) {
        unsafe {
            use winapi::um::processthreadsapi::*;

            let tls_index = TlsAlloc();
            if tls_index == TLS_OUT_OF_INDEXES {
                panic!("{}", std::io::Error::last_os_error());
            }

            bencher.bench(|| TlsGetValue(tls_index));

            TlsFree(tls_index);
        }
    }

    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        any(target_os = "linux", target_os = "macos", target_os = "windows"),
    ))]
    #[divan::bench]
    fn asm() -> usize {
        unsafe {
            let result: usize;

            #[cfg(all(target_arch = "x86_64", any(target_os = "macos", target_os = "windows")))]
            std::arch::asm!(
                "mov {}, gs",
                out(reg) result,
                options(nostack, nomem, preserves_flags)
            );

            #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
            std::arch::asm!(
                "mov {}, fs",
                out(reg) result,
                options(nostack, nomem, preserves_flags)
            );

            // https://developer.arm.com/documentation/ddi0595/2021-12/AArch64-Registers/TPIDRRO-EL0--EL0-Read-Only-Software-Thread-ID-Register?lang=en
            #[cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "windows")))]
            std::arch::asm!(
                "mrs {}, tpidrro_el0",
                out(reg) result,
                options(nostack, nomem, preserves_flags)
            );

            // https://developer.arm.com/documentation/ddi0595/2021-12/AArch64-Registers/TPIDR-EL0--EL0-Read-Write-Software-Thread-ID-Register?lang=en
            #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
            std::arch::asm!(
                "mrs {}, tpidr_el0",
                out(reg) result,
                options(nostack, nomem, preserves_flags)
            );

            result
        }
    }
}
