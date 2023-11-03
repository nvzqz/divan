use std::collections::BTreeSet;

use divan::{black_box, Bencher};
use fastrand::Rng;
use ordsearch::OrderedCollection;

fn main() {
    divan::main();
}

const SIZES: &[usize] =
    &[1, 2, 8, 16, 64, 512, 4 * 1024, 16 * 1024, 64 * 1024, 256 * 1024, 1024 * 1024];

fn gen_inputs(len: usize) -> (impl FnMut() -> Vec<u64>, u64) {
    let mut rng = Rng::with_seed(len as u64);
    let target = rng.u64(..);

    let f = move || {
        let mut buf = Vec::new();
        let has_target = rng.bool();

        let range = if has_target {
            buf.push(target);
            1..len
        } else {
            0..len
        };

        buf.extend(range.map(|_| loop {
            let n = rng.u64(..);
            if n != target {
                return n;
            }
        }));

        buf.sort_unstable();
        buf
    };

    (f, target)
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn linear<const N: usize>(bencher: Bencher) {
    let (gen_inputs, target) = gen_inputs(N);

    bencher
        .counter(N)
        .with_inputs(gen_inputs)
        .bench_local_refs(|buf| _ = black_box(buf.iter().find(|&&v| v == target)))
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn binary<const N: usize>(bencher: Bencher) {
    let (gen_inputs, target) = gen_inputs(N);

    bencher
        .counter(N)
        .with_inputs(gen_inputs)
        .bench_local_refs(|buf| _ = black_box(buf.binary_search_by(|v| v.cmp(&target))))
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn btree_set<const N: usize>(bencher: Bencher) {
    let (mut gen_inputs, target) = gen_inputs(N);

    bencher
        .counter(N)
        .with_inputs(|| -> BTreeSet<u64> { gen_inputs().into_iter().collect() })
        .bench_local_refs(|btree| _ = black_box(btree.get(&target)))
}

#[divan::bench(consts = SIZES, max_time = 1)]
fn ordsearch<const N: usize>(bencher: Bencher) {
    let (mut gen_inputs, target) = gen_inputs(N);

    bencher
        .counter(N)
        .with_inputs(|| OrderedCollection::from_sorted_iter(gen_inputs()))
        .bench_local_refs(|ord_col| _ = black_box(ord_col.find_gte(target)))
}
