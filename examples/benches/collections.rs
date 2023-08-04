use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

use divan::{black_box, Bencher};

fn main() {
    divan::main();
}

mod util {
    use super::*;

    pub fn collect_nums<T: FromIterator<i32>>() -> T {
        black_box(0..100).collect()
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

#[divan::bench(types = [
    Vec<i32>,
    VecDeque<i32>,
    BinaryHeap<i32>,
    HashSet<i32>,
])]
fn with_capacity<T: util::WithCapacity>() -> T {
    // TODO: Make capacity be a provided value.
    let capacity = black_box(100);
    T::with_capacity(capacity)
}

#[divan::bench(types = [
    Vec<i32>,
    VecDeque<i32>,
    LinkedList<i32>,
    BinaryHeap<i32>,
    HashSet<i32>,
    BTreeSet<i32>,
])]
fn from_iter<T: FromIterator<i32>>() -> T {
    util::collect_nums()
}

#[divan::bench(types = [
    Vec<i32>,
    VecDeque<i32>,
    LinkedList<i32>,
    BinaryHeap<i32>,
    HashSet<i32>,
    BTreeSet<i32>,
])]
fn drop<T: FromIterator<i32>>(bencher: Bencher) {
    bencher.with_inputs(from_iter::<T>).bench_values(std::mem::drop);
}

#[divan::bench(types = [
    Vec<i32>,
    VecDeque<i32>,
    LinkedList<i32>,
    BinaryHeap<i32>,
    HashSet<i32>,
    BTreeSet<i32>,
])]
fn clear<T: FromIterator<i32> + util::Clear>(bencher: Bencher) {
    bencher.with_inputs(from_iter::<T>).bench_refs(T::clear);
}
