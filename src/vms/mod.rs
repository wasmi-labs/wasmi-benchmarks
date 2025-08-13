pub use self::stitch::Stitch;
pub use self::tinywasm::Tinywasm;
pub use self::wasm3::Wasm3;
pub use self::wasmer::Wasmer;
pub use self::wasmi_new::WasmiNew;
pub use self::wasmi_old::WasmiOld;
pub use self::wasmtime::{Strategy as WasmtimeStrategy, Wasmtime};
use benchmark_utils::BenchRuntime;

mod stitch;
mod tinywasm;
mod wasm3;
mod wasmer;
mod wasmi_new;
mod wasmi_old;
mod wasmtime;

/// Returns the Wasm runtimes with a set of configurations to test.
pub fn vms_under_test() -> Vec<Box<dyn BenchRuntime>> {
    use self::wasmi_new::Validation;
    vec![
        Box::new(WasmiOld),
        Box::new(WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Eager,
            validation: Validation::Checked,
        }),
        Box::new(WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::LazyTranslation,
            validation: Validation::Checked,
        }),
        Box::new(WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Lazy,
            validation: Validation::Checked,
        }),
        Box::new(WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Lazy,
            validation: Validation::Unchecked,
        }),
        Box::new(Tinywasm),
        Box::new(Wasm3 {
            compilation_mode: wasm3::CompilationMode::Eager,
        }),
        Box::new(Wasm3 {
            compilation_mode: wasm3::CompilationMode::Lazy,
        }),
        Box::new(Stitch),
        Box::new(Wasmtime {
            strategy: WasmtimeStrategy::Cranelift,
        }),
        Box::new(Wasmtime {
            strategy: WasmtimeStrategy::Winch,
        }),
        Box::new(Wasmtime {
            strategy: WasmtimeStrategy::Pulley,
        }),
        Box::new(Wasmer {
            compiler: wasmer::WasmerCompiler::Cranelift,
        }),
        Box::new(Wasmer {
            compiler: wasmer::WasmerCompiler::Singlepass,
        }),
        Box::new(Wasmer {
            compiler: wasmer::WasmerCompiler::Wamr,
        }),
    ]
}
