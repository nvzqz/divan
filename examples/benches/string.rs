use divan::Bencher;

fn main() {
    divan::main();
}

enum StringEnc {
    Ascii,
    Unicode,
}

use StringEnc::*;

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

macro_rules! bench_group {
    ($group:ident, $bench:expr) => {
        mod $group {
            use super::*;

            #[divan::bench]
            fn ascii(bencher: Bencher) {
                let bench: fn(Bencher<'_, _>) = $bench;
                bench(bencher.with_inputs(Ascii.string_generator()));
            }

            #[divan::bench]
            fn unicode(bencher: Bencher) {
                let bench: fn(Bencher<'_, _>) = $bench;
                bench(bencher.with_inputs(Unicode.string_generator()));
            }
        }
    };
}

macro_rules! bench_values {
    ($group:ident, $benched:expr) => {
        bench_group!($group, |bencher| bencher.bench_values($benched));
    };
}

macro_rules! bench_refs {
    ($group:ident, $benched:expr) => {
        bench_group!($group, |bencher| bencher.bench_refs($benched));
    };
}

bench_refs!(clear, |s: &mut String| s.clear());
bench_values!(drop, |s: String| drop(s));

bench_refs!(make_ascii_lowercase, |s: &mut String| s.make_ascii_lowercase());
bench_refs!(make_ascii_uppercase, |s: &mut String| s.make_ascii_uppercase());

bench_refs!(to_ascii_lowercase, |s: &mut String| s.to_ascii_lowercase());
bench_refs!(to_ascii_uppercase, |s: &mut String| s.to_ascii_uppercase());

bench_refs!(to_lowercase, |s: &mut String| s.to_lowercase());
bench_refs!(to_uppercase, |s: &mut String| s.to_uppercase());
