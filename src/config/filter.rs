use regex::Regex;

use crate::util::split_vec::SplitVec;

/// Filters which benchmark/group to run based on its path.
pub(crate) enum Filter {
    Regex(Regex),
    Exact(String),
}

impl Filter {
    fn is_match(&self, s: &str) -> bool {
        match self {
            Self::Regex(regex) => regex.is_match(s),
            Self::Exact(exact) => exact == s,
        }
    }
}

/// Collection of positive and negative filters.
///
/// Positive filters indicate that a benchmark/group path should be included,
/// whereas negative filters exclude entries. Negative filters are placed before
/// positive filters because they have priority.
#[derive(Default)]
pub(crate) struct FilterSet {
    filters: SplitVec<Filter>,
}

impl FilterSet {
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.filters.reserve_exact(additional);
    }

    pub fn insert(&mut self, filter: Filter, positive: bool) {
        self.filters.insert(filter, positive);
    }

    /// Returns `true` if a benchmark/group path matches these filters, and thus
    /// the entry should be included.
    ///
    /// Negative filters are prioritized over positive filters.
    pub fn is_match(&self, entry_path: &str) -> bool {
        let filters = self.filters.all();
        let positive_start = self.filters.split_index();

        // If any filter matches, return whether it was positive or negative.
        // Negative filters are placed before positive filters because they have
        // priority.
        if let Some(index) = filters.iter().position(|f| f.is_match(entry_path)) {
            return index >= positive_start;
        }

        // Otherwise succeed only if there are no positive filters.
        filters.len() == positive_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty filter sets should match all strings.
    #[test]
    fn empty() {
        let filters = FilterSet::default();
        assert!(filters.is_match("abc"));
        assert!(filters.is_match("123"));
    }

    mod single {
        use super::*;

        #[test]
        fn positive_exact() {
            let mut filters = FilterSet::default();

            filters.insert(Filter::Exact("abc".into()), true);

            assert!(filters.is_match("abc"));
            assert!(!filters.is_match("ab"));
            assert!(!filters.is_match("abcd"));
        }

        #[test]
        fn negative_exact() {
            let mut filters = FilterSet::default();

            filters.insert(Filter::Exact("abc".into()), false);

            assert!(!filters.is_match("abc"));
            assert!(filters.is_match("ab"));
            assert!(filters.is_match("abcd"));
        }

        #[test]
        fn positive_regex() {
            let mut filters = FilterSet::default();
            let regex = Regex::new("abc.*123").unwrap();

            filters.insert(Filter::Regex(regex), true);

            assert!(!filters.is_match("abc"));
            assert!(filters.is_match("abc123"));
            assert!(filters.is_match("abc::123"));
        }

        #[test]
        fn negative_regex() {
            let mut filters = FilterSet::default();
            let regex = Regex::new("abc.*123").unwrap();

            filters.insert(Filter::Regex(regex), false);

            assert!(filters.is_match("abc"));
            assert!(!filters.is_match("abc123"));
            assert!(!filters.is_match("abc::123"));
        }
    }

    /// Multiple positive filters should not be restrictive, whereas negative
    /// filters are increasingly restrictive.
    mod multi {
        use super::*;

        #[test]
        fn exact() {
            let mut filters = FilterSet::default();

            filters.insert(Filter::Exact("abc".into()), true);
            filters.insert(Filter::Exact("123".into()), true);

            assert!(filters.is_match("abc"));
            assert!(filters.is_match("123"));
            assert!(!filters.is_match("xyz"));
        }
    }

    /// Negative filters override positive filters.
    mod overridden {
        use super::*;

        #[test]
        fn exact() {
            let mut filters = FilterSet::default();

            filters.insert(Filter::Exact("abc".into()), true);
            filters.insert(Filter::Exact("abc".into()), false);

            assert!(!filters.is_match("abc"));
        }

        #[test]
        fn regex() {
            let mut filters = FilterSet::default();
            let regex = Regex::new("abc.*123").unwrap();

            filters.insert(Filter::Regex(regex.clone()), true);
            filters.insert(Filter::Regex(regex), false);

            assert!(!filters.is_match("abc::123"));
            assert!(!filters.is_match("123::abc"));
        }
    }
}
