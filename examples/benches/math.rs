//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench math
//! ```

use divan::black_box;
use std::collections::{BTreeMap, HashMap};

fn main() {
    divan::main();
}

#[divan::bench]
fn add() -> i32 {
    black_box(2) + black_box(1)
}

#[divan::bench]
#[ignore]
fn sub() -> i32 {
    black_box(2) - black_box(1)
}

#[divan::bench]
fn mul() -> i32 {
    black_box(2) * black_box(1)
}

#[divan::bench]
fn div() -> i32 {
    black_box(2) / black_box(1)
}

#[divan::bench]
fn rem() -> i32 {
    black_box(2) % black_box(1)
}

// 1, 1, 2, 3, 5, ...
mod fibonacci {
    use super::*;

    const VALUES: &[u64] = &[0, 5, 10, 20, 30, 40];

    // O(n)
    #[divan::bench(args = VALUES)]
    fn iterative(n: u64) -> u64 {
        let mut previous = 1;
        let mut current = 1;

        for _ in 2..=n {
            let next = previous + current;
            previous = current;
            current = next;
        }

        current
    }

    // O(2^n)
    #[divan::bench(args = VALUES, max_time = 1)]
    fn recursive(n: u64) -> u64 {
        if n <= 1 {
            1
        } else {
            recursive(n - 2) + recursive(n - 1)
        }
    }

    trait Map: Default {
        fn get(&self, key: u64) -> Option<u64>;
        fn set(&mut self, key: u64, value: u64);
    }

    impl Map for HashMap<u64, u64> {
        fn get(&self, key: u64) -> Option<u64> {
            self.get(&key).copied()
        }

        fn set(&mut self, key: u64, value: u64) {
            self.insert(key, value);
        }
    }

    impl Map for BTreeMap<u64, u64> {
        fn get(&self, key: u64) -> Option<u64> {
            self.get(&key).copied()
        }

        fn set(&mut self, key: u64, value: u64) {
            self.insert(key, value);
        }
    }

    // O(n)
    #[divan::bench(
        types = [BTreeMap<u64, u64>, HashMap<u64, u64>],
        args = VALUES,
    )]
    fn recursive_memoized<M: Map>(n: u64) -> u64 {
        fn fibonacci<M: Map>(n: u64, cache: &mut M) -> u64 {
            if let Some(result) = cache.get(n) {
                return result;
            }

            if n <= 1 {
                return 1;
            }

            let result = fibonacci(n - 2, cache) + fibonacci(n - 1, cache);
            cache.set(n, result);
            result
        }

        fibonacci(n, &mut M::default())
    }
}
