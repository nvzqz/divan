use divan::{black_box, counter::Bytes, Bencher};

fn main() {
    divan::main();
}

const LENS: &[usize] = &[0, 1, 4, 32, 1024];

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
)]
fn clear<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(String::clear);
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn drop<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_values(std::mem::drop);
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn validate_utf8<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher.with_inputs(|| gen.gen_string(N)).input_counter(|s| Bytes(s.len())).bench_refs(|s| {
        let bytes = black_box(s.as_bytes());
        _ = black_box(std::str::from_utf8(bytes));
    });
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn char_count<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(|s| s.chars().count());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn make_ascii_lowercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(|s| s.make_ascii_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn make_ascii_uppercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(|s| s.make_ascii_uppercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_ascii_lowercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(|s| s.to_ascii_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_ascii_uppercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(|s| s.to_ascii_uppercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_lowercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(|s| s.to_lowercase());
}

#[divan::bench(
    types = [Ascii, Unicode],
    consts = LENS,
)]
fn to_uppercase<G: GenString, const N: usize>(bencher: Bencher) {
    let mut gen = G::default();
    bencher
        .with_inputs(|| gen.gen_string(N))
        .input_counter(|s| Bytes(s.len()))
        .bench_refs(|s| s.to_uppercase());
}
