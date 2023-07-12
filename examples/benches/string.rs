use divan::Bencher;

fn main() {
    divan::main();
}

/// Utilities to generate deterministic pseudorandom strings.
mod gen {
    pub use fastrand::Rng;

    const LEN: usize = 100;

    pub fn rng() -> Rng {
        Rng::with_seed(42)
    }

    pub fn any(rng: &mut Rng) -> String {
        (0..LEN).map(|_| rng.char(..)).collect()
    }

    pub fn alphanumeric(rng: &mut Rng) -> String {
        (0..LEN).map(|_| rng.alphanumeric()).collect()
    }
}

mod unicode {
    use super::*;

    #[divan::bench]
    fn to_lowercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::any(&mut rng), |s| s.to_lowercase());
    }

    #[divan::bench]
    fn to_uppercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::any(&mut rng), |s| s.to_uppercase());
    }

    #[divan::bench]
    fn to_ascii_lowercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::any(&mut rng), |s| s.to_ascii_lowercase());
    }

    #[divan::bench]
    fn to_ascii_uppercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::any(&mut rng), |s| s.to_ascii_uppercase());
    }

    #[divan::bench]
    fn make_ascii_lowercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::any(&mut rng), |s| s.make_ascii_lowercase());
    }

    #[divan::bench]
    fn make_ascii_uppercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::any(&mut rng), |s| s.make_ascii_uppercase());
    }
}

mod ascii {
    use super::*;

    #[divan::bench]
    fn to_lowercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::alphanumeric(&mut rng), |s| s.to_lowercase());
    }

    #[divan::bench]
    fn to_uppercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::alphanumeric(&mut rng), |s| s.to_uppercase());
    }

    #[divan::bench]
    fn to_ascii_lowercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::alphanumeric(&mut rng), |s| s.to_ascii_lowercase());
    }

    #[divan::bench]
    fn to_ascii_uppercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::alphanumeric(&mut rng), |s| s.to_ascii_uppercase());
    }

    #[divan::bench]
    fn make_ascii_lowercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::alphanumeric(&mut rng), |s| s.make_ascii_lowercase());
    }

    #[divan::bench]
    fn make_ascii_uppercase(bencher: Bencher) {
        let mut rng = gen::rng();
        bencher.bench_with_refs(|| gen::alphanumeric(&mut rng), |s| s.make_ascii_uppercase());
    }
}
