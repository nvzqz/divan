use std::collections::BTreeSet;

use divan::{black_box, Bencher};
use fastrand::Rng;
use ordsearch::OrderedCollection;

fn main() {
    divan::main();
}

const SIZES: &[usize] =
    &[1, 2, 8, 16, 64, 512, 4 * 1024, 16 * 1024, 64 * 1024, 256 * 1024, 1024 * 1024];

fn gen_inputs(len: usize) -> impl FnMut() -> (Vec<u64>, u64) {
    let mut rng = Rng::with_seed(len as u64);

    move || {
        let mut haystack = Vec::new();

        let needle = rng.u64(..);
        let has_needle = rng.bool();

        let range = if has_needle {
            haystack.push(needle);
            1..len
        } else {
            0..len
        };

        haystack.extend(range.map(|_| loop {
            let n = rng.u64(..);
            if n != needle {
                return n;
            }
        }));

        haystack.sort_unstable();

        (haystack, needle)
    }
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn linear<const N: usize>(bencher: Bencher) {
    bencher
        .counter(N)
        .with_inputs(gen_inputs(N))
        .bench_local_refs(|(haystack, needle)| haystack.iter().find(|v| **v == *needle).copied())
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn binary<const N: usize>(bencher: Bencher) {
    bencher
        .counter(N)
        .with_inputs(gen_inputs(N))
        .bench_local_refs(|(haystack, needle)| haystack.binary_search_by(|v| v.cmp(needle)))
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn btree_set<const N: usize>(bencher: Bencher) {
    let mut gen_inputs = gen_inputs(N);

    bencher
        .counter(N)
        .with_inputs(|| -> (BTreeSet<u64>, u64) {
            let (haystack, needle) = gen_inputs();
            (haystack.into_iter().collect(), needle)
        })
        .bench_local_refs(|(haystack, needle)| haystack.get(needle).copied())
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn ordsearch<const N: usize>(bencher: Bencher) {
    let mut gen_inputs = gen_inputs(N);

    bencher
        .counter(N)
        .with_inputs(|| {
            let (haystack, needle) = gen_inputs();
            (OrderedCollection::from_sorted_iter(haystack), needle)
        })
        .bench_local_refs(|(haystack, needle)| _ = black_box(haystack.find_gte(*needle)))
}
