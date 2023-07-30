use divan::Bencher;
use rayon::slice::ParallelSliceMut;

fn main() {
    divan::main();
}

/// Functions that generate deterministic values.
mod gen {
    pub const LEN: usize = 100_000;

    pub fn rand_int_generator() -> impl FnMut() -> i32 {
        let mut rng = fastrand::Rng::with_seed(42);
        move || rng.i32(..)
    }

    pub fn rand_int_vec_generator() -> impl FnMut() -> Vec<i32> {
        let mut rand_int_generator = rand_int_generator();
        move || (0..LEN).map(|_| rand_int_generator()).collect()
    }

    pub fn sorted_int_vec_generator() -> impl FnMut() -> Vec<i32> {
        move || (0..LEN).map(|i| i as i32).collect()
    }
}

#[divan::bench_group(max_time = 5)]
mod random {
    use super::*;

    #[divan::bench]
    fn sort(bencher: Bencher) {
        bencher.bench_with_refs(gen::rand_int_vec_generator(), |v| v.sort());
    }

    #[divan::bench]
    fn sort_unstable(bencher: Bencher) {
        bencher.bench_with_refs(gen::rand_int_vec_generator(), |v| v.sort_unstable());
    }

    #[divan::bench]
    fn par_sort(bencher: Bencher) {
        bencher.bench_with_refs(gen::rand_int_vec_generator(), |v| v.par_sort());
    }

    #[divan::bench]
    fn par_sort_unstable(bencher: Bencher) {
        bencher.bench_with_refs(gen::rand_int_vec_generator(), |v| v.par_sort_unstable());
    }
}

#[divan::bench_group(max_time = 5)]
mod sorted {
    use super::*;

    #[divan::bench]
    fn sort(bencher: Bencher) {
        bencher.bench_with_refs(gen::sorted_int_vec_generator(), |v| v.sort());
    }

    #[divan::bench]
    fn sort_unstable(bencher: Bencher) {
        bencher.bench_with_refs(gen::sorted_int_vec_generator(), |v| v.sort_unstable());
    }

    #[divan::bench]
    fn par_sort(bencher: Bencher) {
        bencher.bench_with_refs(gen::sorted_int_vec_generator(), |v| v.par_sort());
    }

    #[divan::bench]
    fn par_sort_unstable(bencher: Bencher) {
        bencher.bench_with_refs(gen::sorted_int_vec_generator(), |v| v.par_sort_unstable());
    }
}
