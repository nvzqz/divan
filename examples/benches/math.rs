use std::hint::black_box as bb;

fn main() {
    divan::main();
}

#[divan::bench]
fn add() -> i32 {
    bb(2) + bb(1)
}

#[divan::bench]
fn sub() -> i32 {
    bb(2) - bb(1)
}

#[divan::bench]
fn mul() -> i32 {
    bb(2) * bb(1)
}

#[divan::bench]
fn div() -> i32 {
    bb(2) / bb(1)
}

#[divan::bench]
fn rem() -> i32 {
    bb(2) % bb(1)
}
