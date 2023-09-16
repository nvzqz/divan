//! Utilities for benchmarks. Not directly runnable.

use std::collections::{BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};

use divan::black_box;

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
