#![crate_type = "dylib"]

use benchmark_utils::BenchRuntime;

/// Returns the Wasm runtimes with a set of configurations to test.
pub fn vms_under_test() -> Vec<Box<dyn BenchRuntime>> {
    vec![
        Box::new(rt_wasmi_031::Wasmi031),
        Box::new(rt_wasmi_032::Wasmi032),
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::Eager,
            validation: rt_wasmi_v1::Validation::Checked,
        }),
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::LazyTranslation,
            validation: rt_wasmi_v1::Validation::Checked,
        }),
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::Lazy,
            validation: rt_wasmi_v1::Validation::Checked,
        }),
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::Lazy,
            validation: rt_wasmi_v1::Validation::Unchecked,
        }),
        Box::new(rt_tinywasm::Tinywasm),
        Box::new(rt_wasm3::Wasm3 {
            compilation_mode: rt_wasm3::CompilationMode::Eager,
        }),
        Box::new(rt_wasm3::Wasm3 {
            compilation_mode: rt_wasm3::CompilationMode::Lazy,
        }),
        Box::new(rt_stitch::Stitch),
        #[cfg(feature = "wasmtime-cranelift")]
        Box::new(rt_wasmtime::Wasmtime {
            strategy: rt_wasmtime::Strategy::Cranelift,
        }),
        #[cfg(feature = "wasmtime-winch")]
        Box::new(rt_wasmtime::Wasmtime {
            strategy: rt_wasmtime::Strategy::Winch,
        }),
        #[cfg(feature = "wasmtime-pulley")]
        Box::new(rt_wasmtime::Wasmtime {
            strategy: rt_wasmtime::Strategy::Pulley,
        }),
        #[cfg(feature = "wasmer-cranelift")]
        Box::new(rt_wasmer::Wasmer {
            compiler: rt_wasmer::WasmerCompiler::Cranelift,
        }),
        #[cfg(feature = "wasmer-singlepass")]
        Box::new(rt_wasmer::Wasmer {
            compiler: rt_wasmer::WasmerCompiler::Singlepass,
        }),
    ]
}
