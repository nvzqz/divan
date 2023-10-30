# Divan Internal Benchmarks

This crate demonstrates how to use [Divan] to benchmark internals of a crate by
benchmarking the internals of Divan.

These can be benchmarked locally by running:

```sh
git clone https://github.com/nvzqz/divan.git
cd divan

cargo bench -q -p internal_benches
```

As of this writing, the output on my machine is:

```txt
divan                             fastest  │ slowest  │ median   │ mean     │ samples │ iters
╰─ time                                    │          │          │          │         │
   ╰─ timer                                │          │          │          │         │
      ├─ get_tsc                  0.158 ns │ 0.202 ns │ 0.161 ns │ 0.162 ns │ 100     │ 1638400
      ╰─ measure                           │          │          │          │         │
         ├─ precision             89.58 µs │ 221.5 µs │ 201.9 µs │ 184.5 µs │ 100     │ 100
         ╰─ sample_loop_overhead  314.2 µs │ 342.5 µs │ 314.5 µs │ 317.1 µs │ 100     │ 100
```

[divan]: https://github.com/nvzqz/divan
