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
}

#[divan::bench_group(name = "Vec")]
mod vec {
    use super::*;

    #[divan::bench]
    fn default() -> Vec<i32> {
        Default::default()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.bench_with_values(from_iter, std::mem::drop);
    }

    #[divan::bench]
    fn with_capacity() -> Vec<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        Vec::with_capacity(capacity)
    }

    #[divan::bench]
    fn from_iter() -> Vec<i32> {
        // TODO: Make size be a provided value.
        util::collect_nums()
    }

    #[divan::bench]
    fn copy_from_slice(bencher: Bencher) {
        let src = from_iter();
        let mut dst = vec![0; src.len()];

        bencher.bench(|| black_box(&mut dst).copy_from_slice(black_box(&src)))
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.bench_with_refs(from_iter, Vec::clear);
    }
}

#[divan::bench_group(name = "VecDeque")]
mod vec_deque {
    use super::*;

    #[divan::bench]
    fn default() -> VecDeque<i32> {
        Default::default()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.bench_with_values(from_iter, std::mem::drop);
    }

    #[divan::bench]
    fn with_capacity() -> VecDeque<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        VecDeque::with_capacity(capacity)
    }

    #[divan::bench]
    fn from_iter() -> VecDeque<i32> {
        // TODO: Make size be a provided value.
        util::collect_nums()
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.bench_with_refs(from_iter, VecDeque::clear);
    }
}

#[divan::bench_group(name = "LinkedList")]
mod linked_list {
    use super::*;

    #[divan::bench]
    fn default() -> LinkedList<i32> {
        Default::default()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.bench_with_values(from_iter, std::mem::drop);
    }

    #[divan::bench]
    fn from_iter() -> LinkedList<i32> {
        // TODO: Make size be a provided value.
        util::collect_nums()
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.bench_with_refs(from_iter, LinkedList::clear);
    }
}

#[divan::bench_group(name = "BinaryHeap")]
mod binary_heap {
    use super::*;

    #[divan::bench]
    fn default() -> BinaryHeap<i32> {
        Default::default()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.bench_with_values(from_iter, std::mem::drop);
    }

    #[divan::bench]
    fn with_capacity() -> BinaryHeap<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        BinaryHeap::with_capacity(capacity)
    }

    #[divan::bench]
    fn from_iter() -> BinaryHeap<i32> {
        // TODO: Make size be a provided value.
        util::collect_nums()
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.bench_with_refs(from_iter, BinaryHeap::clear);
    }
}

#[divan::bench_group(name = "HashSet")]
mod hash_set {
    use super::*;

    #[divan::bench]
    fn default() -> HashSet<i32> {
        Default::default()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.bench_with_values(from_iter, std::mem::drop);
    }

    #[divan::bench]
    fn with_capacity() -> HashSet<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        HashSet::with_capacity(capacity)
    }

    #[divan::bench]
    fn from_iter() -> HashSet<i32> {
        // TODO: Make size be a provided value.
        util::collect_nums()
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.bench_with_refs(from_iter, HashSet::clear);
    }
}

#[divan::bench_group(name = "BTreeSet")]
mod btree_set {
    use super::*;

    #[divan::bench]
    fn default() -> BTreeSet<i32> {
        Default::default()
    }

    #[divan::bench]
    fn drop(bencher: Bencher) {
        bencher.bench_with_values(from_iter, std::mem::drop);
    }

    #[divan::bench]
    fn from_iter() -> BTreeSet<i32> {
        // TODO: Make size be a provided value.
        util::collect_nums()
    }

    #[divan::bench]
    fn clear(bencher: Bencher) {
        bencher.bench_with_refs(from_iter, BTreeSet::clear);
    }
}
