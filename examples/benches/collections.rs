use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

use divan::{black_box, Bencher};

fn main() {
    divan::main();
}

fn collect_nums<T: FromIterator<i32>>() -> T {
    black_box(0..100).collect()
}

mod default {
    use super::*;

    #[divan::bench(name = "Vec")]
    fn vec() -> Vec<i32> {
        Default::default()
    }

    #[divan::bench(name = "VecDeque")]
    fn vec_deque() -> VecDeque<i32> {
        Default::default()
    }

    #[divan::bench(name = "LinkedList")]
    fn linked_list() -> LinkedList<i32> {
        Default::default()
    }

    #[divan::bench(name = "BinaryHeap")]
    fn binary_heap() -> BinaryHeap<i32> {
        Default::default()
    }

    #[divan::bench(name = "HashSet")]
    fn hash_set() -> HashSet<i32> {
        Default::default()
    }

    #[divan::bench(name = "BTreeSet")]
    fn btree_set() -> BTreeSet<i32> {
        Default::default()
    }
}

mod with_capacity {
    use super::*;

    #[divan::bench(name = "Vec")]
    fn vec() -> Vec<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        Vec::with_capacity(capacity)
    }

    #[divan::bench(name = "VecDeque")]
    fn vec_deque() -> VecDeque<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        VecDeque::with_capacity(capacity)
    }

    #[divan::bench(name = "BinaryHeap")]
    fn binary_heap() -> BinaryHeap<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        BinaryHeap::with_capacity(capacity)
    }

    #[divan::bench(name = "HashSet")]
    fn hash_set() -> HashSet<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = black_box(100);
        HashSet::with_capacity(capacity)
    }
}

mod from_iter {
    use super::*;

    #[divan::bench(name = "Vec")]
    fn vec() -> Vec<i32> {
        collect_nums()
    }

    #[divan::bench(name = "VecDeque")]
    fn vec_deque() -> VecDeque<i32> {
        collect_nums()
    }

    #[divan::bench(name = "LinkedList")]
    fn linked_list() -> LinkedList<i32> {
        collect_nums()
    }

    #[divan::bench(name = "BinaryHeap")]
    fn binary_heap() -> BinaryHeap<i32> {
        collect_nums()
    }

    #[divan::bench(name = "HashSet")]
    fn hash_set() -> HashSet<i32> {
        collect_nums()
    }

    #[divan::bench(name = "BTreeSet")]
    fn btree_set() -> BTreeSet<i32> {
        collect_nums()
    }
}

mod drop {
    use super::*;

    #[divan::bench(name = "Vec")]
    fn vec(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_values(drop::<Vec<i32>>);
    }

    #[divan::bench(name = "VecDeque")]
    fn vec_deque(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_values(drop::<VecDeque<i32>>);
    }

    #[divan::bench(name = "LinkedList")]
    fn linked_list(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_values(drop::<LinkedList<i32>>);
    }

    #[divan::bench(name = "BinaryHeap")]
    fn binary_heap(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_values(drop::<BinaryHeap<i32>>);
    }

    #[divan::bench(name = "HashSet")]
    fn hash_set(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_values(drop::<HashSet<i32>>);
    }

    #[divan::bench(name = "BTreeSet")]
    fn btree_set(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_values(drop::<BTreeSet<i32>>);
    }
}

mod clear {
    use super::*;

    #[divan::bench(name = "Vec")]
    fn vec(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_refs(Vec::<i32>::clear);
    }

    #[divan::bench(name = "VecDeque")]
    fn vec_deque(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_refs(VecDeque::<i32>::clear);
    }

    #[divan::bench(name = "LinkedList")]
    fn linked_list(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_refs(LinkedList::<i32>::clear);
    }

    #[divan::bench(name = "BinaryHeap")]
    fn binary_heap(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_refs(BinaryHeap::<i32>::clear);
    }

    #[divan::bench(name = "HashSet")]
    fn hash_set(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_refs(HashSet::<i32>::clear);
    }

    #[divan::bench(name = "BTreeSet")]
    fn btree_set(bencher: Bencher) {
        bencher.with_inputs(collect_nums).bench_refs(BTreeSet::<i32>::clear);
    }
}
