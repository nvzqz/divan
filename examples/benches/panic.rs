use std::{hint::black_box, panic};

fn main() {
    // Silence panics.
    panic::set_hook(Box::new(|_| {}));

    divan::main();
}

#[divan::bench]
#[track_caller]
fn caller_location() -> &'static panic::Location<'static> {
    panic::Location::caller()
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
