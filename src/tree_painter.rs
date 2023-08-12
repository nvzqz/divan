//! Happy little trees.

use std::{io::Write, iter::repeat};

use crate::{
    counter::BytesFormat,
    stats::{Stats, StatsSet},
};

/// Paints tree-style output using box-drawing characters.
pub(crate) struct TreePainter {
    /// The maximum number of characters taken by a name and its prefix. Emitted
    /// information should be left-padded to start at this column.
    max_name_span: usize,

    column_widths: [usize; TreeColumn::COUNT],

    depth: usize,

    /// The current prefix to the name and content, e.g.
    /// <code>│       │   </code> for three levels of nesting with the second
    /// level being on the last node.
    current_prefix: String,

    /// Buffer for writing to before printing to stdout.
    write_buf: String,
}

impl TreePainter {
    pub fn new(max_name_span: usize, column_widths: usize) -> Self {
        Self {
            max_name_span,
            column_widths: [column_widths; TreeColumn::COUNT],
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
            "├── "
        } else {
            "╰── "
        };
        buf.extend([self.current_prefix.as_str(), branch, name]);

        // Right-pad name if `has_columns`
        if has_columns {
            let max_span = self.max_name_span;
            let buf_len = buf.chars().count();
            let pad_len = 1 + max_span.saturating_sub(buf_len);
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
            self.current_prefix.push_str(if !is_last { "│   " } else { "    " });
        }
    }

    /// Exit the current parent node.
    pub fn finish_parent(&mut self) {
        self.depth -= 1;

        // Improve legibility for multiple top-level parents.
        if self.depth == 0 {
            println!();
        }

        // The prefix is extended by 4 `char`s at a time.
        let new_prefix_len = {
            let mut iter = self.current_prefix.chars();
            _ = iter.by_ref().rev().nth(3);
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

        let branch = if !is_last { "├── " } else { "╰── " };
        buf.extend([self.current_prefix.as_str(), branch, name]);

        // Right-pad buffer.
        {
            let max_span = self.max_name_span;
            let buf_len = buf.chars().count();
            let pad_len = 1 + max_span.saturating_sub(buf_len);
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

        let branch = if !is_last { "├── " } else { "╰── " };
        buf.extend([self.current_prefix.as_str(), branch, name]);

        // Right-pad buffer if this leaf will have info displayed.
        if has_columns {
            let max_span = self.max_name_span;
            let buf_len = buf.chars().count();
            let pad_len = 1 + max_span.saturating_sub(buf_len);
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

        // Write time stats.
        TreeColumnData::from_fn(|column| column.get_stat(&stats.time).to_string())
            .as_ref::<str>()
            .write(buf, &mut self.column_widths);

        println!("{buf}");

        // Write counter stats.
        if let Some(counter_stats) = &stats.counter {
            buf.clear();
            buf.push_str(&self.current_prefix);

            if !is_last {
                buf.push('│');
            }

            // Right-pad buffer.
            let pad_len = {
                let buf_len = buf.chars().count();
                1 + self.max_name_span.saturating_sub(buf_len)
            };
            buf.extend(repeat(' ').take(pad_len));

            TreeColumnData::from_fn(|column| {
                column
                    .get_stat(counter_stats)
                    .display_throughput(*column.get_stat(&stats.time), bytes_format)
                    .to_string()
            })
            .as_ref::<str>()
            .write(buf, &mut self.column_widths);

            println!("{buf}");
        }
    }

    fn has_columns(&self) -> bool {
        !self.column_widths.iter().all(|&w| w == 0)
    }
}

/// Columns of the table next to the tree.
#[derive(Clone, Copy)]
pub(crate) enum TreeColumn {
    Fastest,
    Slowest,
    Median,
    Mean,
}

impl TreeColumn {
    const COUNT: usize = 4;

    const ALL: [Self; Self::COUNT] = [Self::Fastest, Self::Slowest, Self::Median, Self::Mean];

    fn name(self) -> &'static str {
        match self {
            Self::Fastest => "fastest",
            Self::Slowest => "slowest",
            Self::Median => "median",
            Self::Mean => "mean",
        }
    }

    #[inline]
    fn get_stat<T>(self, stats: &StatsSet<T>) -> &T {
        match self {
            Self::Fastest => &stats.fastest,
            Self::Slowest => &stats.slowest,
            Self::Median => &stats.median,
            Self::Mean => &stats.mean,
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
