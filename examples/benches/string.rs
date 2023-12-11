//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench string
//! ```

use divan::{
    black_box, black_box_drop,
    counter::{BytesCount, CharsCount},
    AllocProfiler, Bencher,
};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

const LENS: &[usize] = &[0, 8, 64, 1024];

#[derive(Default)]
struct Ascii {
    rng: fastrand::Rng,
}

#[derive(Default)]
struct Unicode {
    rng: fastrand::Rng,
}

trait GenString: Default {
    fn gen_string(&mut self, char_count: usize) -> String;
}

impl GenString for Ascii {
    fn gen_string(&mut self, char_count: usize) -> String {
        (0..char_count).map(|_| self.rng.alphanumeric()).collect()
    }
}

impl GenString for Unicode {
    fn gen_string(&mut self, char_count: usize) -> String {
        (0..char_count).map(|_| self.rng.char(..)).collect()
    }
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
    max_time = 1,
)]
fn clear<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(String::clear);
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn drop<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_values(std::mem::drop);
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn validate_utf8<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| {
            let bytes = black_box(s.as_bytes());
            black_box_drop(std::str::from_utf8(bytes));
        });
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn char_count<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.chars().count());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn make_ascii_lowercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.make_ascii_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn make_ascii_uppercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.make_ascii_uppercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_ascii_lowercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_ascii_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_ascii_uppercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_ascii_uppercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_lowercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_uppercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(N))
        .with_inputs(|| gen.gen_string(N))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_uppercase());
}
