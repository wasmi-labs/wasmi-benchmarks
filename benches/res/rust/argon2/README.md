# Argon2 - Benchmark Test Case

This is a test case using the [`argon2`] password hashing crate.

Ideally this had a `build.rs` file but for now I built the associated
`benches/res/wasm/argon2.wasm` file manually via the following steps:

1) `cargo build -p testcase-argon2 --target wasm32-unknown-unknown --profile=wasm`
    - Builds the `testcase-argon2` crate for the `wasm32` target with proper optimizations enabled.
2) `wasm-opt -O3 target/wasm32-unknown-unknown/wasm/testcase_argon2.wasm -o benches/res/wasm/argon2wasm`
    - Optimizes the Wasm file using Binaryen's `wasm-opt` tool on highest optimization settings.
    - Puts it into the directory for use by the benchmark runner.

[`argon2`]: https://crates.io/crates/argon2
[Binaryen's `wasm-opt`]: https://github.com/WebAssembly/binaryen
