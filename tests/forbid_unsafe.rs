// Exhaustively tests that macros work when linting against `unsafe`.

#![forbid(unsafe_code)]

use divan::Bencher;

const CONST_VALUES: [usize; 3] = [1, 5, 10];

#[divan::bench]
fn freestanding() {}

#[divan::bench(types = [i32, &str])]
fn freestanding_generic<T>() {}

#[divan::bench(consts = [1, 5, 10])]
fn freestanding_consts1<const N: usize>() {}

#[divan::bench(consts = CONST_VALUES)]
fn freestanding_consts2<const N: usize>() {}

#[divan::bench]
fn contextual(_: Bencher) {}

#[divan::bench(types = [i32, &str])]
fn contextual_generic<T>(_: Bencher) {}

#[divan::bench(consts = [1, 5, 10])]
fn contextual_consts1<const N: usize>(_: Bencher) {}

#[divan::bench(consts = CONST_VALUES)]
fn contextual_consts2<const N: usize>(_: Bencher) {}

#[divan::bench_group]
mod group {
    use super::*;

    #[divan::bench]
    fn freestanding() {}

    #[divan::bench(types = [i32, &str])]
    fn freestanding_generic<T>() {}

    #[divan::bench(consts = [1, 5, 10])]
    fn freestanding_consts1<const N: usize>() {}

    #[divan::bench(consts = CONST_VALUES)]
    fn freestanding_consts2<const N: usize>() {}

    #[divan::bench]
    fn contextual(_: Bencher) {}

    #[divan::bench(types = [i32, &str])]
    fn contextual_generic<T>(_: Bencher) {}

    #[divan::bench(consts = [1, 5, 10])]
    fn contextual_consts1<const N: usize>(_: Bencher) {}

    #[divan::bench(consts = CONST_VALUES)]
    fn contextual_consts2<const N: usize>(_: Bencher) {}
}
