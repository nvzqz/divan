# Changelog [![crates.io][crate-badge]][crate]

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- [`BytesCount::of_many`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.of_many)
  method similar to [`BytesCount::of`](https://docs.rs/divan/0.1/divan/counter/struct.BytesCount.html#method.of),
  but with a parameter by which to multiply the size of the type.

- [`BytesCount::u64`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.u64),
  [`BytesCount::f64`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.f64),
  and similar methods based on [`BytesCount::of_many`](https://docs.rs/divan/X.Y.Z/divan/counter/struct.BytesCount.html#method.of_many).

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
