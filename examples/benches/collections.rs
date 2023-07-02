use std::{
    collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque},
    hint::black_box as bb,
};

fn main() {
    divan::main();
}

mod vec {
    use super::*;

    #[divan::bench]
    fn default() -> Vec<i32> {
        Default::default()
    }

    #[divan::bench]
    fn with_capacity() -> Vec<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = bb(100);
        Vec::with_capacity(capacity)
    }
}

mod vec_deque {
    use super::*;

    #[divan::bench]
    fn default() -> VecDeque<i32> {
        Default::default()
    }

    #[divan::bench]
    fn with_capacity() -> VecDeque<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = bb(100);
        VecDeque::with_capacity(capacity)
    }
}

mod linked_list {
    use super::*;

    #[divan::bench]
    fn default() -> LinkedList<i32> {
        Default::default()
    }
}

mod binary_heap {
    use super::*;

    #[divan::bench]
    fn default() -> BinaryHeap<i32> {
        Default::default()
    }

    #[divan::bench]
    fn with_capacity() -> BinaryHeap<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = bb(100);
        BinaryHeap::with_capacity(capacity)
    }
}

mod hash_set {
    use super::*;

    #[divan::bench]
    fn default() -> HashSet<i32> {
        Default::default()
    }

    #[divan::bench]
    fn with_capacity() -> HashSet<i32> {
        // TODO: Make capacity be a provided value.
        let capacity = bb(100);
        HashSet::with_capacity(capacity)
    }
}

mod btree_set {
    use super::*;

    #[divan::bench]
    fn default() -> BTreeSet<i32> {
        Default::default()
    }
}
