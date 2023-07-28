//! Benchmarks the [`image`](https://docs.rs/image) crate.

use std::time::Duration;

use divan::{black_box, Bencher};
use image::{GenericImage, ImageBuffer, Rgba};

fn main() {
    divan::main();
}

fn make_image(pixel: Rgba<u8>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    ImageBuffer::from_pixel(2048, 2048, pixel)
}

const MAX_TIME: Duration = Duration::from_secs(3);

// https://github.com/image-rs/image/blob/v0.24.6/benches/copy_from.rs
#[divan::bench(max_time = MAX_TIME, sample_size = 20)]
fn copy_from(bencher: Bencher) {
    let src = make_image(Rgba([255u8, 0, 0, 255]));
    let mut dst = make_image(Rgba([0u8, 0, 0, 255]));

    bencher.bench(|| black_box(&mut dst).copy_from(black_box(&src), 0, 0));
}

/// Baseline for `copy_from`.
#[divan::bench(max_time = MAX_TIME, sample_size = 20)]
fn memcpy(bencher: Bencher) {
    let src = make_image(Rgba([255u8, 0, 0, 255]));
    let mut dst = vec![0; src.len()];

    bencher.bench(|| black_box(&mut dst).copy_from_slice(black_box(&src)));
}
