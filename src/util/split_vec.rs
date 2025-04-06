/// `Vec` partitioned in half.
///
/// This type exists to make `FilterSet` have a smaller footprint and for
/// `FilterSet::is_match` to generate better code.
pub(crate) struct SplitVec<T> {
    items: Vec<T>,
    split_index: usize,
}

impl<T> Default for SplitVec<T> {
    #[inline]
    fn default() -> Self {
        Self { items: Vec::default(), split_index: 0 }
    }
}

impl<T> SplitVec<T> {
    /// Inserts an item to the end of either the first or second half.
    #[inline]
    pub fn insert(&mut self, value: T, after_split: bool) {
        unsafe {
            // Ensure we have at least one slot available.
            self.items.reserve(1);

            let old_len = self.items.len();
            let old_split = self.split_index();

            let start_ptr = self.items.as_mut_ptr();
            let last_ptr = start_ptr.add(old_len);
            let split_ptr = start_ptr.add(old_split);
            let value_slot = if after_split { last_ptr } else { split_ptr };

            // If writing to before the split, then increment the split index
            // and move any value there to the end.
            //
            // NOTE: We can't use `copy_to_nonoverlapping` because both pointers
            // are the same if `old_len` is 0.
            if !after_split {
                split_ptr.copy_to(last_ptr, 1);
                self.set_split_index(old_split + 1);
            }

            value_slot.write(value);
            self.items.set_len(old_len + 1);
        }
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.items.reserve_exact(additional);
    }

    /// Returns the slice of all items.
    #[inline]
    pub fn all(&self) -> &[T] {
        &self.items
    }

    /// Returns the split halves.
    #[inline]
    #[cfg(test)]
    pub fn split(&self) -> (&[T], &[T]) {
        self.items.split_at(self.split_index())
    }

    /// Returns where the halves are split.
    #[inline]
    pub fn split_index(&self) -> usize {
        let index = self.split_index;

        // Optimization hint to remove bounds checks.
        let len = self.items.len();
        unsafe { assert_unchecked!(index <= len, "index {index} out of bounds (len = {len})") }

        index
    }

    /// Sets where the halves are split.
    #[inline]
    pub unsafe fn set_split_index(&mut self, new_index: usize) {
        self.split_index = new_index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn test(vec: &SplitVec<&str>, before: &[&str], after: &[&str]) {
        assert_eq!(vec.split(), (before, after));
    }

    #[test]
    fn before_split() {
        let mut vec = SplitVec::<&str>::default();

        vec.insert("abc", false);
        test(&vec, &["abc"], &[]);

        vec.insert("xyz", false);
        test(&vec, &["abc", "xyz"], &[]);
    }

    #[test]
    fn after_split() {
        let mut vec = SplitVec::<&str>::default();

        vec.insert("abc", true);
        test(&vec, &[], &["abc"]);

        vec.insert("xyz", true);
        test(&vec, &[], &["abc", "xyz"]);
    }

    #[test]
    fn mixed() {
        let mut vec = SplitVec::<&str>::default();

        vec.insert("abc", false);
        test(&vec, &["abc"], &[]);

        vec.insert("xyz", true);
        test(&vec, &["abc"], &["xyz"]);

        vec.insert("123", false);
        test(&vec, &["abc", "123"], &["xyz"]);

        vec.insert("456", true);
        test(&vec, &["abc", "123"], &["xyz", "456"]);

        vec.insert("789", false);
        test(&vec, &["abc", "123", "789"], &["456", "xyz"]);
    }
}
