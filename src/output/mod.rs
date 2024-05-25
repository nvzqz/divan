//! Provides file output, while coupling it to structure the terminal output.

pub mod json;
pub mod tree_painter;

pub(crate) use self::json::json_output;

use crate::{counter::BytesFormat, stats::Stats};
use tree_painter::TreePainter;

pub(crate) enum LeafStat {
    Ignored,
    Empty,
    Benched { stats: Box<Stats>, bytes_format: BytesFormat },
}

pub(crate) enum StatTree {
    Parent { name: String, children: Vec<StatTree> },
    Leaf { name: String, result: LeafStat },
}

pub(crate) struct OutputStats {
    pub(crate) tree: Vec<StatTree>,
    pub(crate) precision: u128,
}

pub(crate) struct StatCollector {
    tree_painter: TreePainter,
}
impl StatCollector {
    pub(crate) fn new(tree_painter: TreePainter) -> Self {
        Self { tree_painter }
    }

    pub(crate) fn parent(
        &mut self,
        name: &str,
        is_last: bool,
        children: impl FnOnce(&mut Self) -> Vec<StatTree>,
    ) -> StatTree {
        self.tree_painter.start_parent(name, is_last);
        let children = (children)(self);
        self.tree_painter.finish_parent();
        StatTree::Parent { name: name.to_owned(), children }
    }

    pub(crate) fn leaf(&mut self, name: &str, is_last: bool, result: LeafStat) -> StatTree {
        match &result {
            LeafStat::Ignored => self.tree_painter.ignore_leaf(name, is_last),
            LeafStat::Empty => {
                self.tree_painter.start_leaf(name, is_last);
                self.tree_painter.finish_empty_leaf();
            }
            LeafStat::Benched { stats, bytes_format } => {
                self.tree_painter.start_leaf(name, is_last);
                self.tree_painter.finish_leaf(is_last, stats, *bytes_format)
            }
        }
        StatTree::Leaf { name: String::from(name), result }
    }
}
