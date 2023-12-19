//! Scratch space for benchmarks.
//!
//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench scratch
//! ```

// Uncomment the code below to measure heap allocations.
// #[global_allocator]
// static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}
