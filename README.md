# Wasmi Benchmarking Suite

This includes execution and compilation benchmarks for the Wasmi interpreter and other Wasm runtimes.

## Runtimes

The following Wasm runtimes and configurations are included.

| Runtime | ID | Configurations | Note |
|:-------:|:---------------|:-----|:---|
| Wasmi v0.31 | `wasmi-v0.31` | default | |
| Wasmi v0.32 | `wasmi-v0.32` | `eager`, `eager.unchecked`, `lazy`, `lazy-translation` | |
| Tinywasm | `tinywasm` | | Included because it was kinda simple. |
| Wasm3 | `wasm3` | `eager`, `lazy` | Generally accepted to be the fastest Wasm interpreter. |
| Wasmtime | `wasmtime` | `cranelift`, `winch` | Winch only works on `x86` platforms. |
| Wasmer | `wasmer` | `cranelift`, `singlepass` | |

## Usage

Run all benchmarks via:

```
cargo bench
```

**Note:** compilation might take some minutes since we are compiling a lot of Wasm runtimes with very high optimization settings.

Filter benchmarks via

- `compile`: for compilation benchmarks.
- `execute`: for execution benchmarks.
- The runtime `ID`, e.g. `wasmi-v0.31` or `wasm3`.
- The runtime configuration on top of the runtime `ID`, e.g. `wasmi-v0.32.lazy`.
- Single test names, e.g. `counter` (execute) or `ffmpeg` (compile)

Examples

Run all runtimes on the `counter` execution benchmark test case:

```
cargo bench execute/counter
```

Run all Wasm3 test cases with its eager compilation configuration:

```
cargo bench wasm3.eager
```
