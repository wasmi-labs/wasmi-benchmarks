#![crate_type = "dylib"]

use benchmark_utils::BenchRuntime;

/// Returns the Wasm runtimes with a set of configurations to test.
pub fn vms_under_test() -> Vec<Box<dyn BenchRuntime>> {
    vec![
        #[cfg(feature = "wasmi-v0-31")]
        Box::new(rt_wasmi_v0_31::WasmiV031),
        #[cfg(feature = "wasmi-v0-32")]
        Box::new(rt_wasmi_v0_32::WasmiV032),
        #[cfg(feature = "wasmi-v1")]
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::Eager,
            validation: rt_wasmi_v1::Validation::Checked,
        }),
        #[cfg(feature = "wasmi-v1")]
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::LazyTranslation,
            validation: rt_wasmi_v1::Validation::Checked,
        }),
        #[cfg(feature = "wasmi-v1")]
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::Lazy,
            validation: rt_wasmi_v1::Validation::Checked,
        }),
        #[cfg(feature = "wasmi-v1")]
        Box::new(rt_wasmi_v1::Wasmi {
            compilation_mode: rt_wasmi_v1::CompilationMode::Lazy,
            validation: rt_wasmi_v1::Validation::Unchecked,
        }),
        #[cfg(feature = "wasmi-v2")]
        Box::new(rt_wasmi_v2::Wasmi {
            compilation_mode: rt_wasmi_v2::CompilationMode::Eager,
            validation: rt_wasmi_v2::Validation::Checked,
        }),
        #[cfg(feature = "wasmi-v2")]
        Box::new(rt_wasmi_v2::Wasmi {
            compilation_mode: rt_wasmi_v2::CompilationMode::LazyTranslation,
            validation: rt_wasmi_v2::Validation::Checked,
        }),
        #[cfg(feature = "wasmi-v2")]
        Box::new(rt_wasmi_v2::Wasmi {
            compilation_mode: rt_wasmi_v2::CompilationMode::Lazy,
            validation: rt_wasmi_v2::Validation::Checked,
        }),
        #[cfg(feature = "wasmi-v2")]
        Box::new(rt_wasmi_v2::Wasmi {
            compilation_mode: rt_wasmi_v2::CompilationMode::Lazy,
            validation: rt_wasmi_v2::Validation::Unchecked,
        }),
        #[cfg(feature = "tinywasm")]
        Box::new(rt_tinywasm::Tinywasm),
        #[cfg(feature = "wasm3")]
        Box::new(rt_wasm3::Wasm3),
        #[cfg(feature = "stitch")]
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
