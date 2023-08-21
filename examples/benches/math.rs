use divan::black_box;

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
    #[divan::bench(consts = VALUES)]
    fn iterative<const N: u64>() -> u64 {
        fn fibonacci(n: u64) -> u64 {
            let mut previous = 1;
            let mut current = 1;

            for _ in 2..=n {
                let next = previous + current;
                previous = current;
                current = next;
            }

            current
        }

        fibonacci(black_box(N))
    }

    // O(2^n)
    #[divan::bench(consts = VALUES, max_time = 1)]
    fn recursive<const N: u64>() -> u64 {
        fn fibonacci(n: u64) -> u64 {
            if n <= 1 {
                1
            } else {
                fibonacci(n - 2) + fibonacci(n - 1)
            }
        }

        fibonacci(black_box(N))
    }
}
