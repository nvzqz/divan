#![allow(dead_code)]

use std::hint::black_box as bb;

fn main() {
    divan::main();
}

#[divan::bench]
fn add() {
    _ = bb(bb(2) + bb(1));
}

#[divan::bench]
fn sub() {
    _ = bb(bb(2) - bb(1));
}

#[divan::bench]
fn mul() {
    _ = bb(bb(2) * bb(1));
}

#[divan::bench]
fn div() {
    _ = bb(bb(2) / bb(1));
}

#[divan::bench]
fn rem() {
    _ = bb(bb(2) % bb(1));
}
