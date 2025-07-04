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
    args = LENS,
    max_time = 1,
)]
fn clear<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(String::clear);
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn drop<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_values(std::mem::drop);
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn validate_utf8<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| {
            let bytes = black_box(s.as_bytes());
            black_box_drop(std::str::from_utf8(bytes));
        });
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn char_count<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.chars().count());
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn make_ascii_lowercase<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.make_ascii_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn make_ascii_uppercase<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.make_ascii_uppercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn to_ascii_lowercase<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_ascii_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn to_ascii_uppercase<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_ascii_uppercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn to_lowercase<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn to_uppercase<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.to_uppercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    args = LENS,
)]
fn matches<G: GenString>(bencher: Bencher, len: usize) {
    let mut gen = G::default();
    // The return value of the closure passed to bench_refs/bench_local_refs
    // is allowed to capture the input lifetime - in this example, its input
    // is a &'a String, and its output a Vec<&'a str>.
    bencher
        .counter(CharsCount::new(len))
        .with_inputs(|| gen.gen_string(len))
        .input_counter(BytesCount::of_str)
        .bench_local_refs(|s| s.matches(|c: char| c.is_ascii_digit()).collect::<Vec<_>>());
}
