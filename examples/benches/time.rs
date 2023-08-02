use std::time::{Instant, SystemTime};

use divan::Bencher;

fn main() {
    divan::main();
}

mod now {
    use super::*;

    #[divan::bench]
    fn instant() -> Instant {
        Instant::now()
    }

    #[divan::bench]
    fn system_time() -> SystemTime {
        SystemTime::now()
    }

    #[divan::bench]
    #[cfg(all(
        not(miri),
        any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"),
    ))]
    fn tsc() -> u64 {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            let timestamp: u64;
            std::arch::asm!(
                "mrs {}, cntvct_el0",
                out(reg) timestamp,
                // Leave off `nomem` because this should be a compiler fence.
                options(nostack, preserves_flags),
            );
            timestamp
        }

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            #[cfg(target_arch = "x86")]
            use std::arch::x86;
            #[cfg(target_arch = "x86_64")]
            use std::arch::x86_64 as x86;

            x86::_rdtsc()
        }
    }
}

mod duration_since {
    use super::*;

    #[divan::bench]
    fn instant(bencher: Bencher) {
        bencher
            .with_inputs(|| [Instant::now(), Instant::now()])
            .bench_values(|[start, end]| end.duration_since(start));
    }

    #[divan::bench]
    fn system_time(bencher: Bencher) {
        bencher
            .with_inputs(|| [SystemTime::now(), SystemTime::now()])
            .bench_values(|[start, end]| end.duration_since(start));
    }
}
