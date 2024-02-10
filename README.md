<div align="center">
    <h1>Divan</h1>
    <a href="https://docs.rs/divan">
        <img src="https://img.shields.io/crates/v/divan.svg?label=docs&color=blue&logo=rust" alt="docs.rs badge">
    </a>
    <a href="https://crates.io/crates/divan">
        <img src="https://img.shields.io/crates/d/divan.svg" alt="Downloads badge">
    </a>
    <a href="https://github.com/nvzqz/divan">
        <img src="https://img.shields.io/github/stars/nvzqz/divan.svg?style=flat&color=black" alt="GitHub stars badge">
    </a>
    <a href="https://github.com/nvzqz/divan/actions/workflows/ci.yml">
        <img src="https://github.com/nvzqz/divan/actions/workflows/ci.yml/badge.svg" alt="CI build status badge">
    </a>
    <p>
        <strong>Comfy bench</strong>marking for Rust projects, brought to you by
        <a href="https://nikolaivazquez.com">Nikolai Vazquez</a>.
    </p>
</div>

## Sponsor

If you or your company find Divan valuable, consider [sponsoring on
GitHub](https://github.com/sponsors/nvzqz) or [donating via
PayPal](https://paypal.me/nvzqz). Sponsorships help me progress on what's
possible with benchmarking in Rust.

## Guide

A guide is being worked on. In the meantime, see:
- [Announcement post](https://nikolaivazquez.com/blog/divan/)
- ["Proving Performance" FOSDEM talk](https://youtu.be/P87C4jNakGs)

## Getting Started

1. Add the following to your project's [`Cargo.toml`](https://doc.rust-lang.org/cargo/reference/manifest.html):

    ```toml
    [dev-dependencies]
    divan = "0.1.11"

    [[bench]]
    name = "example"
    harness = false
    ```

2. Create a benchmarks file at `benches/example.rs`[^1] with your benchmarking code:

    ```rust
    fn main() {
        // Run registered benchmarks.
        divan::main();
    }

    // Define a `fibonacci` function and register it for benchmarking.
    #[divan::bench]
    fn fibonacci() -> u64 {
        fn compute(n: u64) -> u64 {
            if n <= 1 {
                1
            } else {
                compute(n - 2) + compute(n - 1)
            }
        }

        compute(divan::black_box(10))
    }
    ```

3. Run your benchmarks with [`cargo bench`](https://doc.rust-lang.org/cargo/commands/cargo-bench.html):

    ```txt
    example       fastest  │ slowest │ median   │ mean     │ samples │ iters
    ╰─ f​ibonacci  196.1 ns │ 217 ns  │ 197.5 ns │ 198.1 ns │ 100     │ 3200
    ```

See [`#[divan::bench]`][bench_attr] for info on benchmark function registration.

## Examples

Practical example benchmarks can be found in the [`examples/benches`](https://github.com/nvzqz/divan/tree/main/examples/benches)
directory. These can be benchmarked locally by running:

```sh
git clone https://github.com/nvzqz/divan.git
cd divan

cargo bench -q -p examples --all-features
```

More thorough usage examples can be found in the [`#[divan::bench]` documentation][bench_attr_examples].

## License

Like the Rust project, this library may be used under either the
[MIT License](https://github.com/nvzqz/divan/blob/main/LICENSE-MIT) or
[Apache License (Version 2.0)](https://github.com/nvzqz/divan/blob/main/LICENSE-APACHE).

[^1]: Within your crate directory, i.e. [`$CARGO_MANIFEST_DIR`](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)

[bench_attr]: https://docs.rs/divan/latest/divan/attr.bench.html
[bench_attr_examples]: https://docs.rs/divan/latest/divan/attr.bench.html#examples
