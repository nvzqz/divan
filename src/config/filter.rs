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

/// Collection of inclusive and exclusive filters.
///
/// Inclusive filters indicate that a benchmark/group path should be run without
/// running other benchmarks (unless also included).
///
/// Exclusive filters make all matching candidate benchmarks be skipped (even if
/// explicitly included). As a result, they have priority over inclusive
/// filters.
#[derive(Default)]
pub(crate) struct FilterSet {
    /// Stores exclusive filters followed by inclusive filters.
    ///
    /// Exclusive filters are checked first because they have priority and this
    /// simplifies `FilterSet::is_match`.
    filters: SplitVec<Filter>,
}

impl FilterSet {
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.filters.reserve_exact(additional);
    }

    /// Makes this set only match benchmarks for the given filter (or other
    /// inclusive filters).
    #[inline]
    pub fn include(&mut self, filter: Filter) {
        self.insert_filter(filter, true);
    }

    /// Makes this set never match benchmarks for the given filter (or other
    /// exclusive filters).
    #[inline]
    pub fn exclude(&mut self, filter: Filter) {
        self.insert_filter(filter, false);
    }

    fn insert_filter(&mut self, filter: Filter, inclusive: bool) {
        self.filters.insert(filter, inclusive);
    }

    /// Returns `true` if a benchmark/group path matches these filters, and thus
    /// the entry should be included.
    ///
    /// Exclusive filters are prioritized over inclusive filters.
    pub fn is_match(&self, entry_path: &str) -> bool {
        let filters = self.filters.all();
        let inclusive_start = self.filters.split_index();

        // If any filter matches, return whether it was inclusive or exclusive.
        // Exclusive filters are checked before inclusive filters because they
        // have priority.
        if let Some(index) = filters.iter().position(|f| f.is_match(entry_path)) {
            return index >= inclusive_start;
        }

        // Otherwise succeed only if there are no inclusive filters.
        filters.len() == inclusive_start
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
        fn inclusive_exact() {
            let mut filters = FilterSet::default();

            filters.include(Filter::Exact("abc".into()));

            assert!(filters.is_match("abc"));
            assert!(!filters.is_match("ab"));
            assert!(!filters.is_match("abcd"));
        }

        #[test]
        fn exclusive_exact() {
            let mut filters = FilterSet::default();

            filters.exclude(Filter::Exact("abc".into()));

            assert!(!filters.is_match("abc"));
            assert!(filters.is_match("ab"));
            assert!(filters.is_match("abcd"));
        }

        #[test]
        fn inclusive_regex() {
            let mut filters = FilterSet::default();
            let regex = Regex::new("abc.*123").unwrap();

            filters.include(Filter::Regex(regex));

            assert!(!filters.is_match("abc"));
            assert!(filters.is_match("abc123"));
            assert!(filters.is_match("abc::123"));
        }

        #[test]
        fn exclusive_regex() {
            let mut filters = FilterSet::default();
            let regex = Regex::new("abc.*123").unwrap();

            filters.exclude(Filter::Regex(regex));

            assert!(filters.is_match("abc"));
            assert!(!filters.is_match("abc123"));
            assert!(!filters.is_match("abc::123"));
        }
    }

    /// Multiple inclusive filters should not be restrictive, whereas exclusive
    /// filters are increasingly restrictive.
    mod multi {
        use super::*;

        #[test]
        fn exact() {
            let mut filters = FilterSet::default();

            filters.include(Filter::Exact("abc".into()));
            filters.include(Filter::Exact("123".into()));

            assert!(filters.is_match("abc"));
            assert!(filters.is_match("123"));
            assert!(!filters.is_match("xyz"));
        }
    }

    /// Exclusive filters override inclusive filters.
    mod overridden {
        use super::*;

        #[test]
        fn exact() {
            let mut filters = FilterSet::default();

            filters.include(Filter::Exact("abc".into()));
            filters.exclude(Filter::Exact("abc".into()));

            assert!(!filters.is_match("abc"));
        }

        #[test]
        fn regex() {
            let mut filters = FilterSet::default();
            let regex = Regex::new("abc.*123").unwrap();

            filters.include(Filter::Regex(regex.clone()));
            filters.exclude(Filter::Regex(regex));

            assert!(!filters.is_match("abc::123"));
            assert!(!filters.is_match("123::abc"));
        }
    }
}
