pub use self::stitch::Stitch;
pub use self::tinywasm::Tinywasm;
pub use self::wasm3::Wasm3;
pub use self::wasmer::Wasmer;
pub use self::wasmi_new::WasmiNew;
pub use self::wasmi_old::WasmiOld;
pub use self::wasmtime::{Strategy as WasmtimeStrategy, Wasmtime};
use crate::utils::TestFilter;
use ::wasmi_new::ModuleImportsIter;

mod stitch;
mod tinywasm;
mod wasm3;
mod wasmer;
mod wasmi_new;
mod wasmi_old;
mod wasmtime;

/// A Wasm runtime that is capable of being benchmarked.
pub trait BenchVm {
    /// Returns the name of the Wasm runtime and its configuration.
    fn name(&self) -> &'static str;

    /// Returns the [`TestFilter`] which applies to the Wasm runtime and its configuration.
    fn test_filter(&self) -> TestFilter {
        TestFilter::default()
    }

    /// Compiles the `wasm` using the Wasm runtime and its configuration.
    fn compile(&self, wasm: &[u8], imports: ModuleImportsIter);

    /// Loads a Wasm module instance using the Wasm runtime and its configuration.
    ///
    /// The returned Wasm module instance can then be used to issue calls.
    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime>;

    /// Runs the given Coremark Wasm test and returns the result.
    fn coremark(&self, wasm: &[u8]) -> f32;
}

/// The module instance of a Wasm runtime that is capable of being benchmarked.
pub trait BenchRuntime {
    /// Calls the callable Wasm runtime module instance.
    fn call(&mut self, input: i64);
}

/// Returns the Wasm runtimes with a set of configurations to test.
pub fn vms_under_test() -> Vec<Box<dyn BenchVm>> {
    use self::wasmi_new::Validation;
    vec![
        Box::new(WasmiOld),
        Box::new(WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Eager,
            validation: Validation::Checked,
        }),
        Box::new(WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Eager,
            validation: Validation::Unchecked,
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

fn elapsed_ms() -> u32 {
    use std::time::Instant;
    static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed();
    elapsed.as_millis() as u32
}
