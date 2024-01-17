use std::{
    collections::{hash_map::RandomState, BTreeSet, HashSet},
    hash::BuildHasher,
};

use divan::{black_box_drop, AllocProfiler, Bencher};
use fastrand::Rng;
use ordsearch::OrderedCollection;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::Divan::from_args()
        .items_count(
            // Every benchmark iteration searches for a single element.
            1u32,
        )
        .main();
}

const SIZES: &[usize] =
    &[1, 2, 8, 16, 64, 512, 4 * 1024, 16 * 1024, 64 * 1024, 256 * 1024, 1024 * 1024];

fn gen_inputs(len: usize) -> impl FnMut() -> (Vec<u64>, u64) {
    let mut rng = Rng::with_seed(len as u64);

    move || {
        let haystack: Vec<u64> = {
            // Use `BTreeSet` to ensure result is sorted and has `len` items.
            let mut haystack = BTreeSet::new();

            for _ in 0..len {
                while !haystack.insert(rng.u64(..)) {}
            }

            haystack.into_iter().collect()
        };

        let has_needle = rng.bool();
        let needle = if has_needle {
            *rng.choice(&haystack).unwrap()
        } else {
            loop {
                let n = rng.u64(..);
                if !haystack.contains(&n) {
                    break n;
                }
            }
        };

        assert_eq!(haystack.len(), len);
        (haystack, needle)
    }
}

#[divan::bench(args = SIZES, max_time = 1)]
fn linear(bencher: Bencher, len: usize) {
    bencher
        .with_inputs(gen_inputs(len))
        .bench_local_refs(|(haystack, needle)| haystack.iter().find(|v| **v == *needle).copied())
}

#[divan::bench(args = SIZES, max_time = 1)]
fn binary(bencher: Bencher, len: usize) {
    bencher
        .with_inputs(gen_inputs(len))
        .bench_local_refs(|(haystack, needle)| haystack.binary_search_by(|v| v.cmp(needle)))
}

#[divan::bench(args = SIZES, max_time = 1)]
fn btree_set(bencher: Bencher, len: usize) {
    let mut gen_inputs = gen_inputs(len);

    bencher
        .with_inputs(|| -> (BTreeSet<u64>, u64) {
            let (haystack, needle) = gen_inputs();
            (haystack.into_iter().collect(), needle)
        })
        .bench_local_refs(|(haystack, needle)| haystack.get(needle).copied())
}

/// Local implementation instead of `BuildHasherDefault` to get shorter name in
/// output.
#[derive(Default)]
struct WyHash;

impl BuildHasher for WyHash {
    type Hasher = wyhash::WyHash;

    fn build_hasher(&self) -> Self::Hasher {
        wyhash::WyHash::default()
    }
}

#[divan::bench(
    args = SIZES,
    max_time = 1,
    types = [RandomState, WyHash],
)]
fn hash_set<H>(bencher: Bencher, len: usize)
where
    H: BuildHasher + Default,
{
    let mut gen_inputs = gen_inputs(len);

    bencher
        .with_inputs(|| -> (HashSet<u64, H>, u64) {
            let (haystack, needle) = gen_inputs();
            (haystack.into_iter().collect(), needle)
        })
        .bench_local_refs(|(haystack, needle)| haystack.get(needle).copied())
}

#[divan::bench(args = SIZES, max_time = 1)]
fn ordsearch(bencher: Bencher, len: usize) {
    let mut gen_inputs = gen_inputs(len);

    bencher
        .with_inputs(|| {
            let (haystack, needle) = gen_inputs();
            (OrderedCollection::from_sorted_iter(haystack), needle)
        })
        .bench_local_refs(|(haystack, needle)| black_box_drop(haystack.find_gte(*needle)))
}
