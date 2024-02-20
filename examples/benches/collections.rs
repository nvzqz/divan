//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench collections
//! ```

use divan::{black_box, AllocProfiler, Bencher};
use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

pub fn collect_nums<T: FromIterator<i32>>(n: usize) -> T {
    black_box(0..(n as i32)).collect()
}

pub trait WithCapacity {
    fn with_capacity(c: usize) -> Self;
}

pub trait Clear {
    fn clear(&mut self);
}

pub trait PopFront<T> {
    fn pop_front(&mut self) -> Option<T>;
}

impl<T> PopFront<T> for Vec<T> {
    fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(self.remove(0))
        }
    }
}

impl<T> PopFront<T> for VecDeque<T> {
    fn pop_front(&mut self) -> Option<T> {
        self.pop_front()
    }
}

impl<T> PopFront<T> for LinkedList<T> {
    fn pop_front(&mut self) -> Option<T> {
        self.pop_front()
    }
}

macro_rules! impl_with_capacity {
    ($($t:ident),+) => {
        $(impl WithCapacity for $t<i32> {
            fn with_capacity(c: usize) -> Self {
                $t::with_capacity(c)
            }
        })+
    };
}

macro_rules! impl_clear {
    ($($t:ident),+) => {
        $(impl Clear for $t<i32> {
            fn clear(&mut self) {
                $t::clear(self);
            }
        })+
    };
}

impl_with_capacity!(Vec, VecDeque, BinaryHeap, HashSet);
impl_clear!(Vec, VecDeque, BinaryHeap, HashSet, LinkedList, BTreeSet);

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
fn with_capacity<T: WithCapacity>(bencher: Bencher, len: usize) {
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
    bencher.counter(len).bench(|| collect_nums::<T>(len))
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
    bencher.counter(len).with_inputs(|| collect_nums::<T>(len)).bench_values(std::mem::drop);
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
fn clear<T: FromIterator<i32> + Clear>(bencher: Bencher, len: usize) {
    bencher.counter(len).with_inputs(|| collect_nums::<T>(len)).bench_refs(T::clear);
}

#[divan::bench(
    types = [
        Vec<i32>,
        VecDeque<i32>,
        LinkedList<i32>,
    ],
    args = LENS,
)]
fn pop_front<T: FromIterator<i32> + PopFront<i32>>(bencher: Bencher, len: usize) {
    bencher.counter(len).with_inputs(|| collect_nums::<T>(len)).bench_refs(T::pop_front);
}
