//! Benchmarks the [`image`](https://docs.rs/image) crate.

use divan::{black_box, counter::BytesCount, Bencher};
use image::{GenericImage, ImageBuffer, Rgba};

fn main() {
    divan::main();
}

fn make_image(pixel: Rgba<u8>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    ImageBuffer::from_pixel(2048, 2048, pixel)
}

// https://github.com/image-rs/image/blob/v0.24.6/benches/copy_from.rs
#[divan::bench]
fn copy_from(bencher: Bencher) {
    let src = make_image(Rgba([255u8, 0, 0, 255]));
    let mut dst = make_image(Rgba([0u8, 0, 0, 255]));

    bencher
        .counter(BytesCount::of_slice(&*src))
        .bench(|| black_box(&mut dst).copy_from(black_box(&src), 0, 0));
}

/// Baseline for `copy_from`.
#[divan::bench]
fn memcpy(bencher: Bencher) {
    let src = make_image(Rgba([255u8, 0, 0, 255]));
    let mut dst = vec![0; src.len()];

    bencher
        .counter(BytesCount::of_slice(&*src))
        .bench(|| black_box(&mut dst).copy_from_slice(black_box(&src)));
}
