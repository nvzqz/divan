//! Run with:
//!
//! ```sh
//! cargo bench -q -p examples --bench hash --features hash
//! ```

use digest::Digest;
use divan::AllocProfiler;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

struct Blake3;
struct Blake3Par;
struct Sha1;
struct Sha2_256;
struct Sha2_512;
struct Sha3_256;
struct Sha3_512;

/// [`Hasher::write`] + [`Hasher::finish`].
#[divan::bench(
    types = [
        Blake3,
        Blake3Par,
        fnv::FnvHasher,
        highway::HighwayHasher,
        metrohash::MetroHash128,
        metrohash::MetroHash64,
        seahash::SeaHasher,
        Sha1,
        Sha2_256,
        Sha2_512,
        Sha3_256,
        Sha3_512,
        std::collections::hash_map::DefaultHasher,
        twox_hash::XxHash32,
        twox_hash::XxHash64,
        wyhash::WyHash,
    ],
    args = [0, 8, 64, 1024, 1024 * 1024],
    max_time = 1,
)]
fn hash<H>(bencher: divan::Bencher, len: usize)
where
    H: Hasher,
{
    let bytes: Vec<u8> = {
        let mut rng = fastrand::Rng::new();
        (0..len).map(|_| rng.u8(..)).collect()
    };

    bencher
        .counter(divan::counter::BytesCount::new(len))
        .with_inputs(|| bytes.clone())
        .bench_refs(|bytes| H::hash(bytes));
}

trait Hasher {
    type Hash;

    fn hash(bytes: &[u8]) -> Self::Hash;
}

impl<H: std::hash::Hasher + Default> Hasher for H {
    type Hash = u64;

    fn hash(bytes: &[u8]) -> Self::Hash {
        let mut hasher = H::default();
        hasher.write(bytes);
        hasher.finish()
    }
}

impl Hasher for Blake3 {
    type Hash = [u8; 32];

    fn hash(bytes: &[u8]) -> Self::Hash {
        *blake3::hash(bytes).as_bytes()
    }
}

impl Hasher for Blake3Par {
    type Hash = [u8; 32];

    fn hash(bytes: &[u8]) -> Self::Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(bytes);
        *hasher.finalize().as_bytes()
    }
}

impl Hasher for Sha1 {
    type Hash = [u8; 20];

    fn hash(bytes: &[u8]) -> Self::Hash {
        sha1::Sha1::new_with_prefix(bytes).finalize().into()
    }
}

impl Hasher for Sha2_256 {
    type Hash = [u8; 32];

    fn hash(bytes: &[u8]) -> Self::Hash {
        sha2::Sha256::new_with_prefix(bytes).finalize().into()
    }
}

impl Hasher for Sha2_512 {
    type Hash = [u8; 64];

    fn hash(bytes: &[u8]) -> Self::Hash {
        sha2::Sha512::new_with_prefix(bytes).finalize().into()
    }
}

impl Hasher for Sha3_256 {
    type Hash = [u8; 32];

    fn hash(bytes: &[u8]) -> Self::Hash {
        sha3::Sha3_256::new_with_prefix(bytes).finalize().into()
    }
}

impl Hasher for Sha3_512 {
    type Hash = [u8; 64];

    fn hash(bytes: &[u8]) -> Self::Hash {
        sha3::Sha3_512::new_with_prefix(bytes).finalize().into()
    }
}
