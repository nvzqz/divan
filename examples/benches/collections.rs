use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

use divan::{black_box, Bencher};

fn main() {
    divan::main();
}

const LENS: &[usize] = &[0, 1, 4, 32, 128, 1024, 8192];

mod util {
    use super::*;

    pub fn collect_nums<T: FromIterator<i32>>(n: usize) -> T {
        black_box(0..(n as i32)).collect()
    }

    pub trait WithCapacity {
        fn with_capacity(c: usize) -> Self;
    }

    pub trait Clear {
        fn clear(&mut self);
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
}

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
fn with_capacity<T: util::WithCapacity, const N: usize>() -> T {
    T::with_capacity(black_box(N))
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
fn from_iter<T: FromIterator<i32>, const N: usize>() -> T {
    util::collect_nums(N)
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
    bencher.with_inputs(from_iter::<T, N>).bench_values(std::mem::drop);
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
fn clear<T: FromIterator<i32> + util::Clear, const N: usize>(bencher: Bencher) {
    bencher.with_inputs(from_iter::<T, N>).bench_refs(T::clear);
}
