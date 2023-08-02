use divan::Bencher;

fn main() {
    divan::main();
}

enum StringEnc {
    Ascii,
    Unicode,
}

impl StringEnc {
    /// Returns a function that generates deterministic pseudorandom strings.
    pub fn string_generator(self) -> impl FnMut() -> String {
        let mut rng = fastrand::Rng::with_seed(42);

        move || {
            let len = 100;
            (0..len)
                .map(|_| match self {
                    StringEnc::Ascii => rng.alphanumeric(),
                    StringEnc::Unicode => rng.char(..),
                })
                .collect()
        }
    }
}

mod unicode {
    use super::*;

    fn string_generator() -> impl FnMut() -> String {
        StringEnc::Unicode.string_generator()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_values(std::mem::drop);
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.clear());
    }

    #[divan::bench]
    fn to_lowercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_lowercase());
    }

    #[divan::bench]
    fn to_uppercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_uppercase());
    }

    #[divan::bench]
    fn to_ascii_lowercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_ascii_lowercase());
    }

    #[divan::bench]
    fn to_ascii_uppercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_ascii_uppercase());
    }

    #[divan::bench]
    fn make_ascii_lowercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.make_ascii_lowercase());
    }

    #[divan::bench]
    fn make_ascii_uppercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.make_ascii_uppercase());
    }
}

mod ascii {
    use super::*;

    fn string_generator() -> impl FnMut() -> String {
        StringEnc::Ascii.string_generator()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_values(std::mem::drop);
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.clear());
    }

    #[divan::bench]
    fn to_lowercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_lowercase());
    }

    #[divan::bench]
    fn to_uppercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_uppercase());
    }

    #[divan::bench]
    fn to_ascii_lowercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_ascii_lowercase());
    }

    #[divan::bench]
    fn to_ascii_uppercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.to_ascii_uppercase());
    }

    #[divan::bench]
    fn make_ascii_lowercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.make_ascii_lowercase());
    }

    #[divan::bench]
    fn make_ascii_uppercase(bencher: Bencher) {
        bencher.with_inputs(string_generator()).bench_refs(|s| s.make_ascii_uppercase());
    }
}
