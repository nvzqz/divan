use std::sync::atomic::*;

use divan::black_box;

fn main() {
    divan::main();
}

// Available parallelism (0), baseline (1), and common CPU core counts.
const THREADS: &[usize] = &[0, 1, 4, 16];

#[divan::bench_group(threads = THREADS)]
mod basic {
    use super::*;

    #[divan::bench]
    fn load() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).load(Ordering::Relaxed)
    }

    #[divan::bench]
    fn store() {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).store(black_box(2), Ordering::Relaxed);
    }
}

#[divan::bench_group(threads = THREADS)]
mod update {
    use super::*;

    #[divan::bench]
    fn fetch_or() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).fetch_or(black_box(1), Ordering::Relaxed)
    }

    #[divan::bench]
    fn fetch_and() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).fetch_and(black_box(1), Ordering::Relaxed)
    }

    #[divan::bench]
    fn fetch_xor() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).fetch_xor(black_box(1), Ordering::Relaxed)
    }

    #[divan::bench]
    fn fetch_nand() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).fetch_nand(black_box(1), Ordering::Relaxed)
    }

    #[divan::bench]
    fn fetch_add() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).fetch_add(black_box(1), Ordering::Relaxed)
    }

    #[divan::bench]
    fn fetch_sub() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        black_box(&N).fetch_sub(black_box(1), Ordering::Relaxed)
    }
}

#[divan::bench_group(threads = THREADS)]
mod compare_exchange {
    use super::*;

    #[divan::bench]
    fn fetch_mul() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        let mut current = black_box(&N).load(Ordering::Relaxed);
        loop {
            match black_box(&N).compare_exchange(
                current,
                current.wrapping_mul(black_box(2)),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return current,
                Err(n) => current = n,
            }
        }
    }

    #[divan::bench]
    fn fetch_div() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        let mut current = black_box(&N).load(Ordering::Relaxed);
        loop {
            match black_box(&N).compare_exchange(
                current,
                current.wrapping_div(black_box(2)),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return current,
                Err(n) => current = n,
            }
        }
    }

    #[divan::bench]
    fn fetch_mod() -> usize {
        static N: AtomicUsize = AtomicUsize::new(1);

        let mut current = black_box(&N).load(Ordering::Relaxed);
        loop {
            match black_box(&N).compare_exchange(
                current,
                current % black_box(2),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return current,
                Err(n) => current = n,
            }
        }
    }
}
