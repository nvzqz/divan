//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench collections
//! ```

use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

use divan::{black_box, AllocProfiler, Bencher};

mod util;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

const LENS: &[usize] = &[0, 8, 64, 1024];

#[divan::bench(types = [
    Vec<i32>,
    VecDeque<i32>,
    LinkedList<i32>,
    BinaryHeap<i32>,
    HashSet<i32>,
    BTreeSet<i32>,
])]
fn default<T: Default>() -> T {
    T::default()
}

#[divan::bench(
    types = [
        Vec<i32>,
        VecDeque<i32>,
        BinaryHeap<i32>,
        HashSet<i32>,
    ],
    consts = LENS,
)]
fn with_capacity<T: util::WithCapacity, const N: usize>(bencher: Bencher) {
    bencher.counter(N).bench(|| T::with_capacity(black_box(N)))
}

#[divan::bench(
    types = [
        Vec<i32>,
        VecDeque<i32>,
        LinkedList<i32>,
        BinaryHeap<i32>,
        HashSet<i32>,
        BTreeSet<i32>,
    ],
    consts = LENS,
)]
fn from_iter<T: FromIterator<i32>, const N: usize>(bencher: Bencher) {
    bencher.counter(N).bench(|| util::collect_nums::<T>(N))
}

#[divan::bench(
    types = [
        Vec<i32>,
        VecDeque<i32>,
        LinkedList<i32>,
        BinaryHeap<i32>,
        HashSet<i32>,
        BTreeSet<i32>,
    ],
    consts = LENS,
)]
fn drop<T: FromIterator<i32>, const N: usize>(bencher: Bencher) {
    bencher.counter(N).with_inputs(|| util::collect_nums::<T>(N)).bench_values(std::mem::drop);
}

#[divan::bench(
    types = [
        Vec<i32>,
        VecDeque<i32>,
        LinkedList<i32>,
        BinaryHeap<i32>,
        HashSet<i32>,
        BTreeSet<i32>,
    ],
    consts = LENS,
    max_time = 1,
)]
fn clear<T: FromIterator<i32> + util::Clear, const N: usize>(bencher: Bencher) {
    bencher.counter(N).with_inputs(|| util::collect_nums::<T>(N)).bench_refs(T::clear);
}
