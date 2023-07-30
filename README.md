# Divan

A statistically-comfy benchmarking library for Rust projects, brought to you by
[Nikolai Vazquez](https://hachyderm.io/@nikolai).

## Getting Started

1. Add the following to your project's [`Cargo.toml`](https://doc.rust-lang.org/cargo/reference/manifest.html):

    ```toml
    [dev-dependencies]
    divan = "0.0.0"

    [[bench]]
    name = "my_benchmark"
    harness = false
    ```

2. Create your benchmarks file at <code>[$CARGO_MANIFEST_DIR]/benches/my_benchmark.rs</code>
with your benchmarking code:

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

    See [`#[divan::bench]`][bench_attr] for info on benchmark function
    registration.

[$CARGO_MANIFEST_DIR]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates

3. Run your benchmarks with [`cargo bench`](https://doc.rust-lang.org/cargo/commands/cargo-bench.html).

## Examples

Practical example benchmarks can be found in the [`examples/benches`](https://github.com/nvzqz/divan/tree/main/examples/benches)
directory. These can be benchmarked locally by running:

```sh
git clone https://github.com/nvzqz/divan.git
cd divan

cargo bench -p examples
```

More thorough usage examples can be found in the [`#[divan::bench]` documentation][bench_attr_examples].

## License

Like the Rust project, this library may be used under either the
[MIT License](https://github.com/nvzqz/divan/blob/main/LICENSE-MIT) or
[Apache License (Version 2.0)](https://github.com/nvzqz/divan/blob/main/LICENSE-APACHE).

[bench_attr]: https://docs.rs/divan/latest/divan/attr.bench.html
[bench_attr_examples]: https://docs.rs/divan/latest/divan/attr.bench.html#examples
