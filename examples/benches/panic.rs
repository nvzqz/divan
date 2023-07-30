//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench panic
//! ```

use std::panic;

use divan::{black_box, black_box_drop, AllocProfiler};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    // Silence panics.
    panic::set_hook(Box::new(|_| {}));

    divan::main();
}

// Available parallelism (0), baseline (1), and common CPU core counts.
const THREADS: &[usize] = &[0, 1, 4, 16];

#[divan::bench]
#[track_caller]
fn caller_location() -> &'static panic::Location<'static> {
    panic::Location::caller()
}

#[divan::bench_group(threads = THREADS)]
mod hook {
    use super::*;

    #[divan::bench]
    fn set() {
        panic::set_hook(Box::new(|_| {}));
    }

    #[divan::bench]
    fn take() -> impl Drop {
        panic::take_hook()
    }

    #[divan::bench]
    fn take_and_drop() {
        black_box_drop(panic::take_hook());
    }
}

mod catch_unwind {
    use super::*;

    #[divan::bench]
    fn panic() -> std::thread::Result<()> {
        let panic: fn() = || panic!();
        panic::catch_unwind(black_box(panic))
    }

    #[divan::bench]
    fn success() -> std::thread::Result<()> {
        let success: fn() = || {};
        panic::catch_unwind(black_box(success))
    }
}
