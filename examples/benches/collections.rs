//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench collections
//! ```

use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

use divan::{AllocProfiler, Bencher};

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
    args = LENS,
)]
fn with_capacity<T: util::WithCapacity>(bencher: Bencher, len: usize) {
    bencher.counter(len).bench(|| T::with_capacity(len))
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
    args = LENS,
)]
fn from_iter<T: FromIterator<i32>>(bencher: Bencher, len: usize) {
    bencher.counter(len).bench(|| util::collect_nums::<T>(len))
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
    args = LENS,
)]
fn drop<T: FromIterator<i32>>(bencher: Bencher, len: usize) {
    bencher.counter(len).with_inputs(|| util::collect_nums::<T>(len)).bench_values(std::mem::drop);
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
    args = LENS,
    max_time = 1,
)]
fn clear<T: FromIterator<i32> + util::Clear>(bencher: Bencher, len: usize) {
    bencher.counter(len).with_inputs(|| util::collect_nums::<T>(len)).bench_refs(T::clear);
}
