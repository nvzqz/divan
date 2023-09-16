//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench hash --features hash
//! ```

fn main() {
    divan::main();
}

/// [`Hasher::write`] + [`Hasher::finish`].
#[divan::bench(
    types = [
        fnv::FnvHasher,
        highway::HighwayHasher,
        metrohash::MetroHash128,
        metrohash::MetroHash64,
        std::collections::hash_map::DefaultHasher,
        twox_hash::XxHash32,
        twox_hash::XxHash64,
        wyhash::WyHash,
    ],
    consts = [0, 8, 64, 1024],
)]
fn hash<H, const L: usize>(bencher: divan::Bencher)
where
    H: std::hash::Hasher + Default,
{
    let bytes: Vec<u8> = {
        let mut rng = fastrand::Rng::new();
        (0..L).map(|_| rng.u8(..)).collect()
    };

    bencher
        .counter(divan::counter::BytesCount::new(L))
        .with_inputs(|| (H::default(), bytes.clone()))
        .bench_refs(|(hasher, bytes)| {
            hasher.write(divan::black_box(&bytes));
            hasher.finish()
        });
}
