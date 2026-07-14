#![crate_type = "dylib"]

use benchmark_utils::Runtime;

/// Returns the Wasm runtimes with a set of configurations to test.
pub fn vms_under_test() -> Vec<Box<dyn Runtime>> {
    let mut rts = Rts::default();
    #[cfg(feature = "wasmi-v0-31")]
    rts.push(rt_wasmi_v0_31::WasmiV031);
    #[cfg(feature = "wasmi-v0-32")]
    rts.push(rt_wasmi_v0_32::WasmiV032);
    #[cfg(feature = "wasmi-v1")]
    push_wasmi_v1_rts(&mut rts);
    #[cfg(feature = "wasmi-v2")]
    push_wasmi_v2_rts(&mut rts);
    #[cfg(feature = "tinywasm")]
    rts.push(rt_tinywasm::Tinywasm);
    #[cfg(feature = "wasm3")]
    rts.push(rt_wasm3::Wasm3 {
        compilation_mode: rt_wasm3::CompilationMode::Eager,
    });
    #[cfg(feature = "wasm3")]
    rts.push(rt_wasm3::Wasm3 {
        compilation_mode: rt_wasm3::CompilationMode::Lazy,
    });
    #[cfg(feature = "stitch")]
    rts.push(rt_stitch::Stitch);
    #[cfg(feature = "spacewasm")]
    rts.push(rt_spacewasm::SpaceWasm);
    #[cfg(feature = "wamr")]
    rts.push(rt_wamr::Wamr);
    #[cfg(feature = "toywasm")]
    rts.push(rt_toywasm::Toywasm);
    #[cfg(feature = "wasmtime-cranelift")]
    rts.push(rt_wasmtime::Wasmtime {
        strategy: rt_wasmtime::Strategy::Cranelift,
    });
    #[cfg(feature = "wasmtime-winch")]
    rts.push(rt_wasmtime::Wasmtime {
        strategy: rt_wasmtime::Strategy::Winch,
    });
    #[cfg(feature = "wasmtime-pulley")]
    rts.push(rt_wasmtime::Wasmtime {
        strategy: rt_wasmtime::Strategy::Pulley,
    });
    #[cfg(feature = "wasmer-cranelift")]
    rts.push(rt_wasmer::Wasmer {
        compiler: rt_wasmer::WasmerCompiler::Cranelift,
    });
    #[cfg(feature = "wasmer-singlepass")]
    rts.push(rt_wasmer::Wasmer {
        compiler: rt_wasmer::WasmerCompiler::Singlepass,
    });
    #[cfg(feature = "wasmedge")]
    rts.push(rt_wasmedge::WasmEdge);
    rts.into_vec()
}

#[derive(Default)]
struct Rts(Vec<Box<dyn Runtime>>);

impl Rts {
    fn push(&mut self, rt: impl Runtime + 'static) {
        self.0.push(Box::new(rt));
    }

    fn into_vec(self) -> Vec<Box<dyn Runtime>> {
        self.0
    }
}

#[cfg(feature = "wasmi-v1")]
fn push_wasmi_v1_rts(rts: &mut Rts) {
    use rt_wasmi_v1::{CompilationMode, Validation};
    for (compilation_mode, validation) in [
        (CompilationMode::Eager, Validation::Checked),
        (CompilationMode::LazyTranslation, Validation::Checked),
        (CompilationMode::Lazy, Validation::Checked),
        (CompilationMode::Lazy, Validation::Unchecked),
    ] {
        rts.push(rt_wasmi_v1::Wasmi {
            compilation_mode,
            validation,
        });
    }
}

#[cfg(feature = "wasmi-v2")]
fn push_wasmi_v2_rts(rts: &mut Rts) {
    use rt_wasmi_v2::{CompilationMode, Validation};
    for (compilation_mode, validation) in [
        (CompilationMode::Eager, Validation::Checked),
        (CompilationMode::LazyTranslation, Validation::Checked),
        (CompilationMode::Lazy, Validation::Checked),
        (CompilationMode::Lazy, Validation::Unchecked),
    ] {
        rts.push(rt_wasmi_v2::Wasmi {
            compilation_mode,
            validation,
        });
    }
}
