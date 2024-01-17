//! Json output.

use crate::{
    alloc::{AllocOp, AllocOpMap, AllocTally},
    counter::KnownCounterKind,
    stats::{Stats, StatsSet},
};

use super::{LeafStat, OutputStats, StatTree};
use serde_json::{json, Value as JsonValue};

fn alloc_tallies_json(m: &AllocOpMap<AllocTally<StatsSet<f64>>>) -> JsonValue {
    JsonValue::Object(
        AllocOp::ALL
            .iter()
            .map(|op| {
                let tally = m.get(*op);
                (
                    op.name().to_owned(),
                    json!({
                        "count" : statset_json(&tally.count, |c| c as usize),
                        "size" : statset_json(&tally.size, |c| c as usize),
                    }),
                )
            })
            .collect(),
    )
}

fn counters_json(counts: &Stats) -> JsonValue {
    JsonValue::Object(
        KnownCounterKind::ALL
            .iter()
            .map(|c| {
                let stats = counts.get_counts(*c);
                (
                    match c {
                        KnownCounterKind::Bytes => "bytes",
                        KnownCounterKind::Chars => "chars",
                        KnownCounterKind::Items => "items",
                    }
                    .to_owned(),
                    stats.map_or(JsonValue::Null, |s| statset_json(s, |c| c as usize)),
                )
            })
            .collect(),
    )
}

fn statset_json<T: Copy>(s: &StatsSet<T>, intrep: impl Fn(T) -> usize) -> JsonValue {
    json!({
        "fastest": intrep(s.fastest),
        "slowest": intrep(s.slowest),
        "median": intrep(s.median),
        "mean": intrep(s.mean),
    })
}

fn from(value: StatTree) -> (String, JsonValue) {
    match value {
        StatTree::Parent { name, children } => {
            (name, JsonValue::Object(children.into_iter().map(from).collect()))
        }
        StatTree::Leaf { name, result } => (
            name,
            match result {
                LeafStat::Ignored => json!("Ignored"),
                LeafStat::Empty => json!("Empty"),
                LeafStat::Benched { stats, bytes_format: _ } => {
                    let s = *stats;
                    json!(
                        {
                            "sample_count": format!("{}", s.sample_count),
                            "iter_count": format!("{}", s.iter_count),
                            "time": statset_json(&s.time, |f| f.picos as usize),
                            "alloc_tallies" : alloc_tallies_json(&s.alloc_tallies),
                            "counters" : counters_json(&s)
                        }
                    )
                }
            },
        ),
    }
}

pub(crate) fn json_output(
    OutputStats { tree, precision }: OutputStats,
    mut out: impl std::io::prelude::Write,
) -> std::io::Result<()> {
    let benchmark_results: JsonValue = JsonValue::Object(tree.into_iter().map(from).collect());
    let pretty = serde_json::to_string_pretty(&json!(
        {
            "precision" : precision,
            "benchmarks" : benchmark_results,
        }
    ))?;
    write!(out, "{}", pretty)
}
