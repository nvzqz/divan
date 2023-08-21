//! Happy little trees.

use std::{io::Write, iter::repeat};

use crate::{
    counter::{AnyCounter, BytesFormat, KnownCounterKind},
    stats::{Stats, StatsSet},
};

const TREE_COL_BUF: usize = 2;

/// Paints tree-style output using box-drawing characters.
pub(crate) struct TreePainter {
    /// The maximum number of characters taken by a name and its prefix. Emitted
    /// information should be left-padded to start at this column.
    max_name_span: usize,

    column_widths: [usize; TreeColumn::COUNT],

    depth: usize,

    /// The current prefix to the name and content, e.g.
    /// <code>│     │  </code> for three levels of nesting with the second level
    /// being on the last node.
    current_prefix: String,

    /// Buffer for writing to before printing to stdout.
    write_buf: String,
}

impl TreePainter {
    pub fn new(max_name_span: usize, column_widths: [usize; TreeColumn::COUNT]) -> Self {
        Self {
            max_name_span,
            column_widths,
            depth: 0,
            current_prefix: String::new(),
            write_buf: String::new(),
        }
    }
}

impl TreePainter {
    /// Enter a parent node.
    pub fn start_parent(&mut self, name: &str, is_last: bool) {
        let is_top_level = self.depth == 0;
        let has_columns = self.has_columns();

        let buf = &mut self.write_buf;
        buf.clear();

        let branch = if is_top_level {
            ""
        } else if !is_last {
            "├─ "
        } else {
            "╰─ "
        };
        buf.extend([self.current_prefix.as_str(), branch, name]);

        // Right-pad name if `has_columns`
        if has_columns {
            let max_span = self.max_name_span;
            let buf_len = buf.chars().count();
            let pad_len = TREE_COL_BUF + max_span.saturating_sub(buf_len);
            buf.extend(repeat(' ').take(pad_len));
        }

        // Write column headings.
        if has_columns && is_top_level {
            let names = TreeColumnData::from_fn(TreeColumn::name);
            names.write(buf, &mut self.column_widths);
        }

        // Write column spacers.
        if has_columns && !is_top_level {
            TreeColumnData([""; TreeColumn::COUNT]).write(buf, &mut self.column_widths);
        }

        println!("{buf}");

        self.depth += 1;

        if !is_top_level {
            self.current_prefix.push_str(if !is_last { "│  " } else { "   " });
        }
    }

    /// Exit the current parent node.
    pub fn finish_parent(&mut self) {
        self.depth -= 1;

        // Improve legibility for multiple top-level parents.
        if self.depth == 0 {
            println!();
        }

        // The prefix is extended by 3 `char`s at a time.
        let new_prefix_len = {
            let mut iter = self.current_prefix.chars();
            _ = iter.by_ref().rev().nth(2);
            iter.as_str().len()
        };
        self.current_prefix.truncate(new_prefix_len);
    }

    /// Indicate that the next child node was ignored.
    ///
    /// This semantically combines start/finish operations.
    pub fn ignore_leaf(&mut self, name: &str, is_last: bool) {
        let has_columns = self.has_columns();

        let buf = &mut self.write_buf;
        buf.clear();

        let branch = if !is_last { "├─ " } else { "╰─ " };
        buf.extend([self.current_prefix.as_str(), branch, name]);

        // Right-pad buffer.
        {
            let max_span = self.max_name_span;
            let buf_len = buf.chars().count();
            let pad_len = TREE_COL_BUF + max_span.saturating_sub(buf_len);
            buf.extend(repeat(' ').take(pad_len));
        }

        if has_columns {
            let mut columns = [""; TreeColumn::COUNT];
            columns[0] = "(ignored)";
            TreeColumnData(columns).write(buf, &mut self.column_widths);
        } else {
            buf.push_str("(ignored)");
        }

        println!("{buf}");
    }

    /// Enter a leaf node.
    pub fn start_leaf(&mut self, name: &str, is_last: bool) {
        let has_columns = self.has_columns();

        let buf = &mut self.write_buf;
        buf.clear();

        let branch = if !is_last { "├─ " } else { "╰─ " };
        buf.extend([self.current_prefix.as_str(), branch, name]);

        // Right-pad buffer if this leaf will have info displayed.
        if has_columns {
            let max_span = self.max_name_span;
            let buf_len = buf.chars().count();
            let pad_len = TREE_COL_BUF + max_span.saturating_sub(buf_len);
            buf.extend(repeat(' ').take(pad_len));
        }

        print!("{buf}");
        _ = std::io::stdout().flush();
    }

    /// Exit the current leaf node.
    pub fn finish_empty_leaf(&mut self) {
        println!();
    }

    /// Exit the current leaf node, emitting statistics.
    pub fn finish_leaf(&mut self, is_last: bool, stats: &Stats, bytes_format: BytesFormat) {
        let buf = &mut self.write_buf;
        buf.clear();

        // Serialize counter stats early so we can resize columns early.
        let serialized_counters = KnownCounterKind::ALL.map(|counter_kind| {
            let counter_stats = stats.get_counts(counter_kind);

            TreeColumn::ALL
                .map(|column| -> Option<String> {
                    let count = *column.get_stat(counter_stats?)?;
                    let time = *column.get_stat(&stats.time)?;

                    Some(
                        AnyCounter::known(counter_kind, count)
                            .display_throughput(time, bytes_format)
                            .to_string(),
                    )
                })
                .map(Option::unwrap_or_default)
        });

        let max_counter_width = serialized_counters
            .iter()
            .flatten()
            .map(|s| s.chars().count())
            .max()
            .unwrap_or_default();

        for column in TreeColumn::time_stats() {
            let width = &mut self.column_widths[column as usize];
            *width = (*width).max(max_counter_width);
        }

        // Write time stats with iter and sample counts.
        TreeColumnData::from_fn(|column| -> String {
            let stat: &dyn ToString = match column {
                TreeColumn::Fastest => &stats.time.fastest,
                TreeColumn::Slowest => &stats.time.slowest,
                TreeColumn::Median => &stats.time.median,
                TreeColumn::Mean => &stats.time.mean,
                TreeColumn::Iters => &stats.iter_count,
                TreeColumn::Samples => &stats.sample_count,
            };
            stat.to_string()
        })
        .as_ref::<str>()
        .write(buf, &mut self.column_widths);

        println!("{buf}");

        // Write counter stats.
        let counter_stats = serialized_counters.map(TreeColumnData);
        for counter_kind in KnownCounterKind::ALL {
            let counter_stats = counter_stats[counter_kind as usize].as_ref::<str>();

            // Skip empty rows.
            if counter_stats.0.iter().all(|s| s.is_empty()) {
                continue;
            }

            buf.clear();
            buf.push_str(&self.current_prefix);

            if !is_last {
                buf.push('│');
            }

            // Right-pad buffer.
            let pad_len = {
                let buf_len = buf.chars().count();
                TREE_COL_BUF + self.max_name_span.saturating_sub(buf_len)
            };
            buf.extend(repeat(' ').take(pad_len));

            counter_stats.write(buf, &mut self.column_widths);
            println!("{buf}");
        }
    }

    fn has_columns(&self) -> bool {
        !self.column_widths.iter().all(|&w| w == 0)
    }
}

/// Columns of the table next to the tree.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeColumn {
    Fastest,
    Slowest,
    Median,
    Mean,
    Iters,
    Samples,
}

impl TreeColumn {
    pub const COUNT: usize = 6;

    pub const ALL: [Self; Self::COUNT] = {
        use TreeColumn::*;
        [Fastest, Slowest, Median, Mean, Iters, Samples]
    };

    #[inline]
    pub fn time_stats() -> impl Iterator<Item = Self> {
        use TreeColumn::*;
        [Fastest, Slowest, Median, Mean].into_iter()
    }

    #[inline]
    pub fn is_last(self) -> bool {
        let [.., last] = Self::ALL;
        self == last
    }

    fn name(self) -> &'static str {
        match self {
            Self::Fastest => "fastest",
            Self::Slowest => "slowest",
            Self::Median => "median",
            Self::Mean => "mean",
            Self::Iters => "iters",
            Self::Samples => "samples",
        }
    }

    #[inline]
    pub fn is_time_stat(self) -> bool {
        use TreeColumn::*;
        matches!(self, Fastest | Slowest | Median | Mean)
    }

    #[inline]
    fn get_stat<T>(self, stats: &StatsSet<T>) -> Option<&T> {
        match self {
            Self::Fastest => Some(&stats.fastest),
            Self::Slowest => Some(&stats.slowest),
            Self::Median => Some(&stats.median),
            Self::Mean => Some(&stats.mean),
            Self::Iters | Self::Samples => None,
        }
    }
}

struct TreeColumnData<T>([T; TreeColumn::COUNT]);

impl<T> TreeColumnData<T> {
    #[inline]
    fn from_fn<F>(f: F) -> Self
    where
        F: FnMut(TreeColumn) -> T,
    {
        Self(TreeColumn::ALL.map(f))
    }
}

impl TreeColumnData<&str> {
    /// Writes the column data into the buffer.
    fn write(&self, buf: &mut String, column_widths: &mut [usize; TreeColumn::COUNT]) {
        for (column, value) in self.0.iter().enumerate() {
            let is_first = column == 0;
            let is_last = column == TreeColumn::COUNT - 1;

            let value_width = value.chars().count();

            // Write separator.
            if !is_first {
                let mut sep = " │ ";

                // Prevent trailing spaces.
                if is_last && value_width == 0 {
                    sep = &sep[..sep.len() - 1];
                };

                buf.push_str(sep);
            }

            buf.push_str(value);

            // Right-pad remaining width or update column width to new maximum.
            if !is_last {
                if let Some(rem_width) = column_widths[column].checked_sub(value_width) {
                    buf.extend(repeat(' ').take(rem_width));
                } else {
                    column_widths[column] = value_width;
                }
            }
        }
    }
}

impl<T> TreeColumnData<T> {
    #[inline]
    fn as_ref<U: ?Sized>(&self) -> TreeColumnData<&U>
    where
        T: AsRef<U>,
    {
        TreeColumnData::from_fn(|column| self.0[column as usize].as_ref())
    }
}
