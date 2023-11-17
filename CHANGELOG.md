# Changelog [![crates.io][crate-badge]][crate]

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Support for NetBSD, DragonFly BSD, and Haiku OS by using pre-`main`.

- Set global thread counts using:
  - [`Divan::threads`](https://docs.rs/divan/X.Y.Z/divan/struct.Divan.html#method.threads)
  - `--threads A B C...` CLI arg
  - `DIVAN_THREADS=A,B,C` env var

  The following example will benchmark across 2, 4, and [available parallelism]
  thread counts:

  ```sh
  DIVAN_THREADS=0,2,4 cargo bench -q -p examples --bench atomic
  ```

- Set global
  [`Counter`s](https://docs.rs/divan/X.Y.Z/divan/counter/trait.Counter.html) at
  runtime using:
  - [`Divan::counter`](https://docs.rs/divan/X.Y.Z/divan/struct.Divan.html#method.counter)
  - [`Divan::items_count`](https://docs.rs/divan/X.Y.Z/divan/struct.Divan.html#method.items_count)
  - [`Divan::bytes_count`](https://docs.rs/divan/X.Y.Z/divan/struct.Divan.html#method.bytes_count)
  - [`Divan::chars_count`](https://docs.rs/divan/X.Y.Z/divan/struct.Divan.html#method.chars_count)
  - `--items-count N` CLI arg
  - `--bytes-count N` CLI arg
  - `--chars-count N` CLI arg
  - `DIVAN_ITEMS_COUNT=N` env var
  - `DIVAN_BYTES_COUNT=N` env var
  - `DIVAN_CHARS_COUNT=N` env var

- `From<C>` for
  [`ItemsCount`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.ItemsCount.html),
  [`BytesCount`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html),
  and
  [`CharsCount`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.CharsCount.html)
  where `C` is `u8`–`u64` or `usize` (via `CountUInt` internally). This provides
  an alternative to the `new` constructor.

- [`BytesCount::of_many`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.of_many)
  method similar to [`BytesCount::of`](https://docs.rs/divan/0.1/divan/counter/struct.BytesCount.html#method.of),
  but with a parameter by which to multiply the size of the type.

- [`BytesCount::u64`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.u64),
  [`BytesCount::f64`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.f64),
  and similar methods based on [`BytesCount::of_many`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.of_many).

### Removed

- [`black_box`] inside benchmark loop when deferring [`Drop`] of outputs. This
  is now done after the loop.

- [`linkme`](https://docs.rs/linkme) dependency in favor of pre-`main` to
  register benchmarks and benchmark groups. This is generally be more portable
  and reliable.

### Changed

- Now calling [`black_box`] at the end of the benchmark loop when deferring use
  of inputs or [`Drop`] of outputs.

## [0.1.2] - 2023-10-28

### Fixed

- Multi-threaded benchmarks being spread across CPUs, instead of pinning the
  main thread to CPU 0 and having all threads inherit the main thread's
  affinity.

## [0.1.1] - 2023-10-25

### Fixed

- Fix using LLD as linker for Linux by using the same pre-`main` approach as
  Windows.

## 0.1.0 - 2023-10-04

Initial release. See [blog post](https://nikolaivazquez.com/blog/divan/).

[crate]:       https://crates.io/crates/divan
[crate-badge]: https://img.shields.io/crates/v/divan.svg

[Unreleased]: https://github.com/nvzqz/divan/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/nvzqz/divan/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/nvzqz/divan/compare/v0.1.0...v0.1.1

[`black_box`]: https://doc.rust-lang.org/std/hint/fn.black_box.html
[`Drop`]: https://doc.rust-lang.org/std/ops/trait.Drop.html
[available parallelism]: https://doc.rust-lang.org/std/thread/fn.available_parallelism.html
