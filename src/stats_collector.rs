use std::{collections::HashMap, error::Error};

use crate::{
    counter::{KnownCounterKind, MaxCountUInt},
    entry::AnyBenchEntry,
    stats::{Stats, StatsSet},
};

/// Collects tree-style statistics of benchmarks
#[derive(Default)]
pub(crate) struct StatsCollector {
    path: Vec<String>,
    stats: HashMap<String, JsonStats>,
}

impl StatsCollector {
    pub(crate) fn enter_group(&mut self, group_name: String) {
        self.path.push(group_name);
    }

    pub(crate) fn leave_group(&mut self) {
        self.path.pop().unwrap();
    }

    pub(crate) fn add(&mut self, stats: Stats, entry: &AnyBenchEntry) {
        self.stats.insert(self.path.join("."), JsonStats::from_stats(stats, entry));
    }

    pub(crate) fn write(&self) -> Result<(), Box<dyn Error>> {
        eprintln!("Printing stats...");

        let mut path = get_cargo_target_dir()?;
        path.push("divan");
        std::fs::create_dir_all(&path)?;

        let time = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)?;
        path.push(time + ".json");

        let writer = std::fs::File::create(path)?;
        serde_json::to_writer(writer, &self.stats)?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
pub(crate) struct JsonStats {
    pub display_name: String,
    pub sample_count: u32,
    pub iter_count: u64,
    pub time: StatsSet<u128>,
    pub counts: HashMap<&'static str, StatsSet<MaxCountUInt>>,
}

impl JsonStats {
    pub(crate) fn from_stats(stats: Stats, entry: &AnyBenchEntry) -> Self {
        Self {
            display_name: entry.display_name().to_string(),
            sample_count: stats.sample_count,
            iter_count: stats.iter_count,
            time: StatsSet {
                fastest: stats.time.fastest.picos,
                slowest: stats.time.slowest.picos,
                median: stats.time.median.picos,
                mean: stats.time.mean.picos,
            },
            counts: {
                let mut map = HashMap::new();
                if let Some(bytes) = &stats.counts[KnownCounterKind::Bytes as usize] {
                    map.insert("bytes", bytes.clone());
                }
                if let Some(chars) = &stats.counts[KnownCounterKind::Chars as usize] {
                    map.insert("chars", chars.clone());
                }
                if let Some(items) = &stats.counts[KnownCounterKind::Items as usize] {
                    map.insert("items", items.clone());
                }
                map
            },
        }
    }
}

fn get_cargo_target_dir() -> Result<std::path::PathBuf, Box<dyn Error>> {
    let out_dir = std::path::PathBuf::from(env!("OUT_DIR"));
    let profile = env!("PROFILE");
    let mut target_dir = None;
    let mut sub_path = out_dir.as_path();

    while let Some(parent) = sub_path.parent() {
        if parent.ends_with(profile) {
            target_dir = Some(parent);
            break;
        }
        sub_path = parent;
    }

    let target_dir = target_dir.ok_or("target directory not found")?;
    Ok(target_dir.parent().unwrap().to_path_buf())
}
