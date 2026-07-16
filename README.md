# Wasmi Benchmarking Suite

This includes execution and compilation benchmarks for the Wasmi interpreter and other Wasm runtimes.

## Runtimes

The following Wasm runtimes and configurations are included.

| Runtime | Kind | Configurations | Description |
|:-------:|:---------------|:-----|:---|
| [`wasmi-v0.31`][Wasmi v0.31] | Interpreter | | The old version of the Wasmi Wasm stack-machine interpreter. |
| [`wasmi-v1`][Wasmi v1] | Interpreter | `eager`, `eager.unchecked`, `lazy`, `lazy.unchecked`, `lazy-translation` | The Wasmi v1 interpreter.  |
| [`wasmi-v2`][Wasmi v2] | Interpreter | `eager`, `eager.unchecked`, `lazy`, `lazy.unchecked`, `lazy-translation` | The new and experimental Wasmi v2 interpreter with its various optimizations. |
| [`wasmtime`][Wasmtime] | JIT, Interpreter | `cranelift`, `winch`, `pulley` | Well established Wasm JIT runtime with cutting edge Wasm features. |
| [`wasmer`][Wasmer] | JIT | `cranelift`, `singlepass` | Universal Wasm JIT runtime with many bindings to other language ecosystems. |
| [`wamr`][WAMR] | Interpreter | | The WebAssembly Micro Runtime (WAMR) fast interpreter. |
| [`wasm3`][Wasm3] | Interpreter | `eager`, `lazy` | A fast and well-established Wasm interpreter. |
| [`stitch`][Stitch] | Interpreter | | Experimental and very fast zero-dependencies Wasm interpreter. |
| [`tinywasm`][Tinywasm] | Interpreter | | Small Wasm interpreter written in pure stable Rust. |
| [`toywasm`][Toywasm] | Interpreter | | Feature-rich in-place WebAssembly interpreter with low memory usage. |
| [`spacewasm`][SpaceWasm] | Interpreter | | NASA JPL's `no_std` Wasm 1.0 (MVP) interpreter for on-board spacecraft use. |
| [`wasmedge`][WasmEdge] | Interpreter | | The interpreter of the WasmEdge Wasm runtime. |

**Missing Wasm runtimes:** [Wain], [V8], [DLR-wasm-interpreter]

[Wasmi v0.31]: https://github.com/wasmi-labs/wasmi/tree/v0.31.2
[Wasmi v1]: https://github.com/wasmi-labs/wasmi/tree/v1.0.9
[Wasmi v2]: https://github.com/wasmi-labs/wasmi/tree/v2.0.0-beta.7
[WAMR]: https://github.com/bytecodealliance/wasm-micro-runtime
[Toywasm]: https://github.com/yamt/toywasm
[Wain]: https://github.com/rhysd/wain
[Tinywasm]: https://github.com/explodingcamera/tinywasm
[Wasm3]: https://github.com/wasm3/wasm3
[Wasmtime]: https://github.com/bytecodealliance/wasmtime
[Wasmer]: https://github.com/wasmerio/wasmer
[Stitch]: https://github.com/makepad/stitch
[SpaceWasm]: https://github.com/nasa/spacewasm
[WasmEdge]: https://github.com/wasmedge/wasmedge
[V8]: https://github.com/v8/v8
[DLR-wasm-interpreter]: https://github.com/DLR-FT/wasm-interpreter

### Configuration Explanation

- `eager`: All function bodies are compiled and validated immediately.
- `eager.unchecked`: Function bodies are compiled eagerly but Wasm validation is skipped.
- `lazy`: Function bodies are only compiled and validated upon first use.
- `lazy.unchecked`: Function bodies are only compiled upon first use and Wasm validation is skipped.
- `lazy-translation`: Function bodies are lazily compiled but eagerly validated.
- `cranelift`: The runtime uses the Cranelift code generator.
- `wasmtime.winch`: Wasmtime's JIT optimized for fast start-up times.
- `wasmer.singlepass`: Wasmer's JIT optimized for fast start-up times.

**Note:** by default runtimes compile and validate function bodies eagerly.

## Usage

Run all benchmarks via:

```
cargo bench
```

**Note:** compilation might take some minutes since we are compiling a lot of Wasm runtimes with very high optimization settings.

Filter benchmarks via

- `startup`: for startup (compile + instantiate) benchmarks.
- `execute`: for execution benchmarks.
- The runtime `ID`, e.g. `wasmi-v0.31` or `wasm3`.
- The runtime configuration on top of the runtime `ID`, e.g. `wasmi-v0.32.lazy`.
- Single test names, e.g. `counter` (execute) or `ffmpeg` (startup)

Examples

Run all runtimes on the `counter` execution benchmark test case:

```
cargo bench execute/counter
```

Run all Wasm3 test cases with its eager compilation configuration:

```
cargo bench wasm3.eager
```

## Test Cases

The Wasmi benchmarking test suite provides the following test cases:

| Mode | Test Case | Notes |
|:----:|:---------:|:------|
| | | |
| `execute` | | |
| | `counter-local` | Simple loop that counts a single local down from some number. |
| | `counter-param` | Simple loop that counts down from some number via a control parameter. |
| | `counter-global` | Simple loop that counts a global down from some number. |
| | `fibonacci-rec` | Recursive fibonacci calculation. Call-intense workload. |
| | `fibonacci-iter` | Iterative fibonacci calculation. Compute intense workload. |
| | `fibonacci-tail` | Tail-call based fibonacci calculation. |
| | `sort`   | Executes Rust's standard [`sort_unstable`](https://doc.rust-lang.org/std/primitive.slice.html#method.sort_unstable) on integers. |
| | `prime_sieve` | Executes a Rust sieve of eratosthenes implementation. |
| | `primes` | Calculates all primes until some number. Uses linear memory for storing known primes. |
| | `matmul` | Naive matrix multiplication implementation. Makes heavy use of linear memory and floats. |
| | `argon2` | Password hashing library. Compute- and memory intense workload. |
| | `bulk-ops` | Show cases the Wasm [`bulk-memory-operations`] proposal. |
| | | |
| `startup` | | |
| | `bz2` | Medium-sized compression library with huge function bodies. (WASI required) |
| | `pulldown-cmark` | Medium-sized markdown renderer. (WASI required) |
| | `spidermonkey` | The firefox Javascript execution engine. (large, WASI required) |
| | `ffmpeg` | Huge multimedia library. (WASI required) |
| | `coremark` | CoreMark benchmarking compilation. (kinda small, no WASI) |
| | `argon2` | Password hashing library. (small, no WASI) |
| | `erc20` | ink! based ERC-20 implementation. (tiny, no WASI) |

[`bulk-memory-operations`]: https://github.com/WebAssembly/bulk-memory-operations

## Coremark

This benchmark suite also contains a Coremark test which can be run via

```
cargo run --profile bench
```

This will run Coremark using all available Wasm VMs and print their 
Coremark scores to the console. Higher scores are better.

## Plotting

In order to run the benchmarks and simultaneously plot diagrams of their results use the following command:

```
cargo criterion --bench criterion --message-format=json | cargo run --bin plot
```

This generates plots in the `target/wasmi-benchmarks` folder for all the benchmark groups.
In order to use this you may need to install `cargo-criterion` via `cargo install cargo-criterion`.

In case you want to collect data first and plot later you can also instead store
the benchmark results into a file and use the file to plot the data later:

```
cargo criterion --bench criterion --message-format=json > results.json
cat results.json | cargo run --bin plot
```

## License

Licensed under either of

  * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
  * MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
