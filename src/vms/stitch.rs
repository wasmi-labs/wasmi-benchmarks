use crate::{CompileTestFilter, ExecuteTestFilter, TestFilter};

use super::{BenchRuntime, BenchVm, elapsed_ms};
use core::slice;
use makepad_stitch::{Engine, ExternVal, Func, Instance, Linker, Module, Store, Val};
use wasmi_new::ModuleImportsIter;

pub struct Stitch;

struct StitchRuntime {
    store: Store,
    _instance: Instance,
    func: Func,
}

impl BenchVm for Stitch {
    fn name(&self) -> &'static str {
        "stitch"
    }

    fn test_filter(&self) -> TestFilter {
        // Due to its reliance on LLVM's sibling calls optimization
        // stitch only works on 64-bit platforms where this optimization
        // is "guaranteed" to be applied.
        let stitch_works = cfg!(target_pointer_width = "64");
        TestFilter {
            execute: ExecuteTestFilter {
                fib_tailrec: false, // stich does not yet support tail calls
                argon2: false,      // stitch currently seems to have a bug while executing
                ..ExecuteTestFilter::set_to(stitch_works)
            },
            compile: CompileTestFilter {
                ffmpeg: false, // function body too large for stitch
                ..CompileTestFilter::set_to(stitch_works)
            },
        }
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        let engine = Engine::new();
        Module::new(&engine, wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let engine = Engine::new();
        let mut store = Store::new(engine);
        let engine = store.engine();
        let module = Module::new(engine, wasm).unwrap();
        let linker = Linker::new();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance.exported_func("run").unwrap();
        Box::new(StitchRuntime {
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let engine = Engine::new();
        let mut store = Store::new(engine);
        let engine = store.engine();
        let module = Module::new(engine, wasm).unwrap();
        let mut linker = Linker::new();
        linker.define(
            "env",
            "clock_ms",
            ExternVal::Func(Func::wrap(&mut store, elapsed_ms)),
        );
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance.exported_func("run").unwrap();
        let mut result = Val::F32(0.0);
        func.call(&mut store, &[], slice::from_mut(&mut result))
            .unwrap();
        result.to_f32().unwrap()
    }
}

impl BenchRuntime for StitchRuntime {
    fn call(&mut self, input: i64) {
        let mut result = Val::I64(0);
        self.func
            .call(
                &mut self.store,
                &[Val::I64(input)],
                slice::from_mut(&mut result),
            )
            .unwrap();
    }
}
