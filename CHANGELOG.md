# Changelog [![crates.io][crate-badge]][crate]

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changes

- Limit [`AllocProfiler`] mean count outputs to 4 significant digits to not be
  very wide and for consistency with other outputs.

## [0.1.6] - 2023-12-13

### Added

- [`AllocProfiler`] allocator that tracks allocation counts and sizes during
  benchmarks.

## [0.1.5] - 2023-12-05

### Added

- [`black_box_drop`](https://docs.rs/divan/0.1.5/divan/fn.black_box_drop.html)
  convenience function for [`black_box`] + [`drop`][drop_fn]. This is useful
  when benchmarking a lazy [`Iterator`] to completion with `for_each`:

  ```rust
  #[divan::bench]
  fn parse_iter() {
      let input: &str = // ...

      Parser::new(input)
          .for_each(divan::black_box_drop);
  }
  ```

## [0.1.4] - 2023-12-02

### Added

- `From` implementations for counters on references to `u8`–`u64` and `usize`,
  such as `From<&u64>` and `From<&&u64>`. This allows for doing:

  ```rust
  bencher
      .with_inputs(|| { ... })
      .input_counter(ItemsCount::from)
      .bench_values(|n| { ... });
  ```

- [`Bencher::count_inputs_as<C>`](https://docs.rs/divan/0.1.4/divan/struct.Bencher.html#method.count_inputs_as)
  method to convert inputs to a `Counter`:

  ```rust
  bencher
      .with_inputs(|| -> usize {
          // ...
      })
      .count_inputs_as::<ItemsCount>()
      .bench_values(|n| -> Vec<usize> {
          (0..n).collect()
      });
  ```

## [0.1.3] - 2023-11-21

### Added

- Convenience shorthand options for `#[divan::bench]` and
  `#[divan::bench_group]` counters:
  - [`bytes_count`](https://docs.rs/divan/0.1.3/divan/attr.bench.html#bytes_count)
    for `counter = BytesCount::from(n)`
  - [`chars_count`](https://docs.rs/divan/0.1.3/divan/attr.bench.html#chars_count)
    for `counter = CharsCount::from(n)`
  - [`items_count`](https://docs.rs/divan/0.1.3/divan/attr.bench.html#items_count)
    for `counter = ItemsCount::from(n)`

- Support for NetBSD, DragonFly BSD, and Haiku OS by using pre-`main`.

- Set global thread counts using:
  - [`Divan::threads`](https://docs.rs/divan/0.1.3/divan/struct.Divan.html#method.threads)
  - `--threads A B C...` CLI arg
  - `DIVAN_THREADS=A,B,C` env var

  The following example will benchmark across 2, 4, and [available parallelism]
  thread counts:

  ```sh
  DIVAN_THREADS=0,2,4 cargo bench -q -p examples --bench atomic
  ```

- Set global
  [`Counter`s](https://docs.rs/divan/0.1.3/divan/counter/trait.Counter.html) at
  runtime using:
  - [`Divan::counter`](https://docs.rs/divan/0.1.3/divan/struct.Divan.html#method.counter)
  - [`Divan::items_count`](https://docs.rs/divan/0.1.3/divan/struct.Divan.html#method.items_count)
  - [`Divan::bytes_count`](https://docs.rs/divan/0.1.3/divan/struct.Divan.html#method.bytes_count)
  - [`Divan::chars_count`](https://docs.rs/divan/0.1.3/divan/struct.Divan.html#method.chars_count)
  - `--items-count N` CLI arg
  - `--bytes-count N` CLI arg
  - `--chars-count N` CLI arg
  - `DIVAN_ITEMS_COUNT=N` env var
  - `DIVAN_BYTES_COUNT=N` env var
  - `DIVAN_CHARS_COUNT=N` env var

- `From<C>` for
  [`ItemsCount`](https://docs.rs/divan/0.1.3/divan/counter/struct.ItemsCount.html),
  [`BytesCount`](https://docs.rs/divan/0.1.3/divan/counter/struct.BytesCount.html),
  and
  [`CharsCount`](https://docs.rs/divan/0.1.3/divan/counter/struct.CharsCount.html)
  where `C` is `u8`–`u64` or `usize` (via `CountUInt` internally). This provides
  an alternative to the `new` constructor.

- [`BytesCount::of_many`](https://docs.rs/divan/0.1.3/divan/counter/struct.BytesCount.html#method.of_many)
  method similar to [`BytesCount::of`](https://docs.rs/divan/0.1/divan/counter/struct.BytesCount.html#method.of),
  but with a parameter by which to multiply the size of the type.

- [`BytesCount::u64`](https://docs.rs/divan/0.1.3/divan/counter/struct.BytesCount.html#method.u64),
  [`BytesCount::f64`](https://docs.rs/divan/0.1.3/divan/counter/struct.BytesCount.html#method.f64),
  and similar methods based on [`BytesCount::of_many`](https://docs.rs/divan/0.1.3/divan/counter/struct.BytesCount.html#method.of_many).

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

[Unreleased]: https://github.com/nvzqz/divan/compare/v0.1.6...HEAD
[0.1.6]: https://github.com/nvzqz/divan/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/nvzqz/divan/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/nvzqz/divan/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/nvzqz/divan/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/nvzqz/divan/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/nvzqz/divan/compare/v0.1.0...v0.1.1

[`AllocProfiler`]: https://docs.rs/divan/0.1/divan/struct.AllocProfiler.html

[`black_box`]: https://doc.rust-lang.org/std/hint/fn.black_box.html
[`Drop`]: https://doc.rust-lang.org/std/ops/trait.Drop.html
[`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
[available parallelism]: https://doc.rust-lang.org/std/thread/fn.available_parallelism.html
[drop_fn]: https://doc.rust-lang.org/std/mem/fn.drop.html
