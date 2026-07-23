//! Golang benchmark format output.

use crate::{
    alloc::AllocOp,
    counter::{BytesFormat, KnownCounterKind},
    stats::Stats,
};

mod picos {
    pub const NANOS: u128 = 1_000;
    pub const SEC: u128 = 1_000_000_000_000;
}

/// Paints output in Golang benchmark format.
///
/// Go benchmark format:
/// BenchmarkName-8         1000000              1234 ns/op            123 B/op          4 allocs/op
///
/// Format breakdown:
/// - BenchmarkName-N (N is number of threads)
/// - Iteration count (total iterations across all samples)
/// - Average time per operation with unit
/// - Optional throughput metrics (B/op for bytes/items, allocs/op for allocations)
pub(crate) struct GolangPainter {
    /// Current path for nested benchmarks
    current_path: Vec<String>,
}

impl GolangPainter {
    pub fn new() -> Self {
        Self { current_path: Vec::new() }
    }

    /// Get the full benchmark name from the current path
    fn get_benchmark_name(&self) -> String {
        if self.current_path.is_empty() {
            "Benchmark".to_string()
        } else {
            format!("Benchmark{}", self.current_path.join("/"))
        }
    }
}

fn rewrite(s: &str) -> String {
    let mut result = String::new();

    for ch in s.chars() {
        if ch.is_whitespace() {
            result.push('_');
        } else if !is_printable(ch) {
            // Escape non-printable characters
            let escaped = format!("{:?}", ch);
            // Remove surrounding quotes from the debug representation
            result.push_str(&escaped[1..escaped.len() - 1]);
        } else {
            result.push(ch);
        }
    }

    result
}

fn is_printable(ch: char) -> bool {
    !ch.is_control() || ch == '\t'
}
impl GolangPainter {
    /// Enter a parent node.
    pub fn start_parent(&mut self, name: &str, _is_last: bool) {
        self.current_path.push(rewrite(name));
    }

    /// Exit the current parent node.
    pub fn finish_parent(&mut self) {
        self.current_path.pop();
    }

    /// Indicate that the next child node was ignored.
    pub fn ignore_leaf(&mut self, name: &str, _is_last: bool) {}

    /// Enter a leaf node.
    pub fn start_leaf(&mut self, name: &str, _is_last: bool) {
        self.current_path.push(rewrite(name));
    }

    /// Exit the current leaf node.
    pub fn finish_empty_leaf(&mut self) {
        self.current_path.pop();
        // In golang format, we don't output anything for empty leaves during listing
    }

    /// Exit the current leaf node, emitting statistics in Golang format.
    pub fn finish_leaf(
        &mut self,
        _is_last: bool,
        stats: &Stats,
        _bytes_format: BytesFormat,
    ) {
        let benchmark_name = self.get_benchmark_name();
        self.current_path.pop();

        // Calculate total iterations
        let total_iters = stats.sample_count as u64 * stats.iter_count;

        // Get mean time in nanoseconds
        let mean_picos = stats.time.mean.picos;
        let mean_ns = mean_picos as f64 / picos::NANOS as f64;

        // Format: BenchmarkName-1         1000000              1234 ns/op
        print!(
            "{:<40} {:>10} {:>20.1} ns/op",
            format!("{}-1", benchmark_name),
            total_iters,
            mean_ns
        );

        // Check for bytes or items throughput
        for counter_kind in [KnownCounterKind::Items, KnownCounterKind::Bytes] {
            if let Some(counter_stats) = stats.get_counts(counter_kind) {
                let count = counter_stats.mean;
                if count > 0 && !stats.time.mean.is_zero() {
                    // Calculate throughput per operation
                    let time_secs =
                        stats.time.mean.picos as f64 / picos::SEC as f64;
                    let throughput_per_sec = count as f64 / time_secs;
                    print!(" {:>20.0} B/op", throughput_per_sec);
                    break;
                }
            }
        }

        // Add allocation information if available
        let alloc_tally = stats.alloc_tallies.get(AllocOp::Alloc);
        if !alloc_tally.count.mean.is_nan() && alloc_tally.count.mean > 0.0 {
            print!(" {:>15.0} allocs/op", alloc_tally.count.mean);
        }

        println!();
    }
}
