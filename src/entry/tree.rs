use std::cmp::Ordering;

use crate::{
    config::SortingAttr,
    entry::{AnyBenchEntry, EntryLocation, EntryMeta, GenericBenchEntry, GroupEntry},
};

/// `BenchEntry` tree organized by path components.
pub(crate) enum EntryTree<'a> {
    /// Benchmark group; parent to leaves and other parents.
    Parent { raw_name: &'a str, group: Option<&'a GroupEntry>, children: Vec<Self> },

    /// Benchmark entry leaf.
    Leaf(AnyBenchEntry<'a>),
}

impl<'a> EntryTree<'a> {
    /// Constructs a tree from an iterator of benchmark entries in the order
    /// they're produced.
    pub fn from_benches<I>(benches: I) -> Vec<Self>
    where
        I: IntoIterator<Item = AnyBenchEntry<'a>>,
    {
        let mut result = Vec::<Self>::new();

        for bench in benches {
            let mut module_path = bench.meta().module_path_components();
            let mut extended_module_path;

            let module_iter: &mut dyn Iterator<Item = &str> = match bench {
                AnyBenchEntry::Bench { .. } => &mut module_path,

                // Generic benchmarks consider their group's raw name to be the
                // last path component.
                AnyBenchEntry::GenericBench(bench) => {
                    extended_module_path = module_path.chain(Some(bench.group.meta.raw_name));
                    &mut extended_module_path
                }
            };

            Self::insert_entry(&mut result, bench, module_iter);
        }

        result
    }

    /// Returns the maximum span for a name in `tree`.
    pub fn max_name_span(tree: &[Self], depth: usize) -> usize {
        tree.iter()
            .map(|node| {
                let node_name_len = node.display_name().chars().count();
                let node_name_span = node_name_len + (depth * 4);

                let children_max = Self::max_name_span(node.children(), depth + 1);

                node_name_span.max(children_max)
            })
            .max()
            .unwrap_or_default()
    }

    /// Inserts the benchmark group into a tree.
    ///
    /// Groups are inserted after tree construction because it prevents having
    /// parents without terminating leaves. Groups that do not match an existing
    /// parent are not inserted.
    pub fn insert_group(mut tree: &mut [Self], group: &'a GroupEntry) {
        // Update `tree` to be the innermost set of subtrees whose parents match
        // `group.module_path`.
        'component: for component in group.meta.module_path_components() {
            for subtree in tree {
                match subtree {
                    EntryTree::Parent { raw_name, children, .. } if component == *raw_name => {
                        tree = children;
                        continue 'component;
                    }
                    _ => {}
                }
            }

            // No matches for this component in any subtrees.
            return;
        }

        // Find the matching tree to insert the group into.
        for subtree in tree {
            match subtree {
                EntryTree::Parent { raw_name, group: slot, .. }
                    if group.meta.raw_name == *raw_name =>
                {
                    *slot = Some(group);
                    return;
                }
                _ => {}
            }
        }
    }

    /// Removes entries from the tree whose paths do not match the filter.
    pub fn retain(tree: &mut Vec<Self>, mut filter: impl FnMut(&str) -> bool) {
        fn retain(
            tree: &mut Vec<EntryTree>,
            parent_path: &str,
            filter: &mut impl FnMut(&str) -> bool,
        ) {
            tree.retain_mut(|subtree| {
                let full_path: String;
                let full_path: &str = if parent_path.is_empty() {
                    subtree.display_name()
                } else {
                    full_path = format!("{parent_path}::{}", subtree.display_name());
                    &full_path
                };

                match subtree {
                    EntryTree::Parent { children, .. } => {
                        retain(children, full_path, filter);
                        !children.is_empty()
                    }
                    EntryTree::Leaf { .. } => filter(full_path),
                }
            });
        }
        retain(tree, "", &mut filter);
    }

    /// Sorts the tree by the given ordering.
    pub fn sort_by_attr(tree: &mut [Self], attr: SortingAttr, reverse: bool) {
        tree.sort_unstable_by(|a, b| {
            let ordering = a.cmp_by_attr(b, attr);
            if reverse {
                ordering.reverse()
            } else {
                ordering
            }
        });
        tree.iter_mut().for_each(|tree| Self::sort_by_attr(tree.children_mut(), attr, reverse));
    }

    fn cmp_by_attr(&self, other: &Self, attr: SortingAttr) -> Ordering {
        for attr in attr.with_tie_breakers() {
            let ordering = match attr {
                SortingAttr::Kind => self.kind().cmp(&other.kind()),
                SortingAttr::Name => self.display_name().cmp(other.display_name()),
                SortingAttr::Location => self.cmp_location(other),
            };
            if ordering.is_ne() {
                return ordering;
            }
        }
        Ordering::Equal
    }

    /// Helper for constructing a tree.
    ///
    /// This uses recursion because the iterative approach runs into limitations
    /// with mutable borrows.
    fn insert_entry(
        tree: &mut Vec<Self>,
        entry: AnyBenchEntry<'a>,
        rem_modules: &mut dyn Iterator<Item = &'a str>,
    ) {
        if let Some(current_module) = rem_modules.next() {
            if let Some(children) = Self::get_children(tree, current_module) {
                Self::insert_entry(children, entry, rem_modules);
            } else {
                tree.push(Self::from_path(entry, current_module, rem_modules));
            }
        } else {
            tree.push(Self::Leaf(entry));
        }
    }

    /// Constructs a sequence of branches from a module path.
    fn from_path(
        entry: AnyBenchEntry<'a>,
        current_module: &'a str,
        rem_modules: &mut dyn Iterator<Item = &'a str>,
    ) -> Self {
        let child = if let Some(next_module) = rem_modules.next() {
            Self::from_path(entry, next_module, rem_modules)
        } else {
            Self::Leaf(entry)
        };
        Self::Parent { raw_name: current_module, group: None, children: vec![child] }
    }

    /// Finds the `Parent.children` for the corresponding module in `tree`.
    fn get_children<'t>(tree: &'t mut [Self], module: &str) -> Option<&'t mut Vec<Self>> {
        tree.iter_mut().find_map(|tree| match tree {
            Self::Parent { raw_name, children, group: _ } if *raw_name == module => Some(children),
            _ => None,
        })
    }

    /// Returns an integer denoting the enum variant.
    ///
    /// This is used instead of `std::mem::Discriminant` because it does not
    /// implement `Ord`.
    pub fn kind(&self) -> i32 {
        // Leaves should appear before parents.
        match self {
            Self::Leaf { .. } => 0,
            Self::Parent { .. } => 1,
        }
    }

    pub fn meta(&self) -> Option<&'a EntryMeta> {
        match self {
            Self::Parent { group, .. } => Some(&(*group)?.meta),
            Self::Leaf(bench) => Some(bench.meta()),
        }
    }

    pub fn raw_name(&self) -> &'a str {
        match self {
            Self::Parent { group: Some(group), .. } => group.meta.raw_name,
            Self::Parent { raw_name, .. } => raw_name,
            Self::Leaf(bench) => bench.raw_name(),
        }
    }

    pub fn display_name(&self) -> &'a str {
        if let Self::Leaf(bench) = self {
            bench.display_name()
        } else if let Some(common) = self.meta() {
            common.display_name
        } else {
            let raw_name = self.raw_name();
            raw_name.strip_prefix("r#").unwrap_or(raw_name)
        }
    }

    /// Returns the location of this entry, group, or the children's earliest
    /// location.
    fn location(&self) -> Option<&'a EntryLocation> {
        if let Some(common) = self.meta() {
            Some(&common.location)
        } else {
            self.children().iter().flat_map(Self::location).min()
        }
    }

    /// Compares location with special consideration for whether this is a
    /// generic benchmark.
    ///
    /// When comparing by location, generic benchmarks use the order in which
    /// their types are specified.
    fn cmp_location(&self, other: &Self) -> Ordering {
        let ordering = self.location().cmp(&other.location());

        match (ordering, self, other) {
            (
                Ordering::Equal,
                Self::Leaf(AnyBenchEntry::GenericBench(this)),
                Self::Leaf(AnyBenchEntry::GenericBench(other)),
            ) => {
                // Compare by address as a proxy for slice index.
                let this: *const GenericBenchEntry = *this;
                let other: *const GenericBenchEntry = *other;
                this.cmp(&other)
            }
            _ => ordering,
        }
    }

    fn children(&self) -> &[Self] {
        match self {
            Self::Leaf { .. } => &[],
            Self::Parent { children, .. } => children,
        }
    }

    fn children_mut(&mut self) -> &mut [Self] {
        match self {
            Self::Leaf { .. } => &mut [],
            Self::Parent { children, .. } => children,
        }
    }
}
