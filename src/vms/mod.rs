pub use self::wasm3::Wasm3;
use benchmark_utils::BenchRuntime;

mod wasm3;

/// Returns the Wasm runtimes with a set of configurations to test.
pub fn vms_under_test() -> Vec<Box<dyn BenchRuntime>> {
    vec![
        Box::new(rt_wasmi_031::WasmiOld),
        Box::new(rt_wasmi::WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Eager,
            validation: rt_wasmi::Validation::Checked,
        }),
        Box::new(rt_wasmi::WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::LazyTranslation,
            validation: rt_wasmi::Validation::Checked,
        }),
        Box::new(rt_wasmi::WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Lazy,
            validation: rt_wasmi::Validation::Checked,
        }),
        Box::new(rt_wasmi::WasmiNew {
            compilation_mode: ::wasmi_new::CompilationMode::Lazy,
            validation: rt_wasmi::Validation::Unchecked,
        }),
        Box::new(rt_tinywasm::Tinywasm),
        Box::new(Wasm3 {
            compilation_mode: wasm3::CompilationMode::Eager,
        }),
        Box::new(Wasm3 {
            compilation_mode: wasm3::CompilationMode::Lazy,
        }),
        Box::new(rt_stitch::Stitch),
        Box::new(rt_wasmtime::Wasmtime {
            strategy: rt_wasmtime::Strategy::Cranelift,
        }),
        Box::new(rt_wasmtime::Wasmtime {
            strategy: rt_wasmtime::Strategy::Winch,
        }),
        Box::new(rt_wasmtime::Wasmtime {
            strategy: rt_wasmtime::Strategy::Pulley,
        }),
        Box::new(rt_wasmer::Wasmer {
            compiler: rt_wasmer::WasmerCompiler::Cranelift,
        }),
        Box::new(rt_wasmer::Wasmer {
            compiler: rt_wasmer::WasmerCompiler::Singlepass,
        }),
        Box::new(rt_wasmer::Wasmer {
            compiler: rt_wasmer::WasmerCompiler::Wamr,
        }),
    ]
}
