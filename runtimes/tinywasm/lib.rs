#![crate_type = "dylib"]

use benchmark_utils::{
    BenchInstance, BenchRuntime, ExecuteTestFilter, ModuleImportsIter, TestFilter, elapsed_ms,
};

pub struct Tinywasm;

struct TinywasmRuntime {
    store: tinywasm::Store,
    _instance: tinywasm::ModuleInstance,
    func: tinywasm::FunctionTyped<i64, i64>,
}

impl BenchRuntime for Tinywasm {
    fn name(&self) -> &'static str {
        "tinywasm"
    }

    fn test_filter(&self) -> TestFilter {
        TestFilter {
            execute: ExecuteTestFilter {
                fib_tailrec: false,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        tinywasm::parse_bytes(wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = tinywasm::Store::default();
        let module = tinywasm::parse_bytes(wasm).unwrap();
        let instance = tinywasm::ModuleInstance::instantiate(&mut store, &module, None).unwrap();
        let func = instance.func::<i64, i64>(&store, "run").unwrap();
        Box::new(TinywasmRuntime {
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = tinywasm::Store::default();
        let module = tinywasm::parse_bytes(wasm).unwrap();
        let mut imports = tinywasm::Imports::new();
        imports.define(
            "env",
            "clock_ms",
            tinywasm::HostFunction::from(&mut store, |_ctx, _arg: ()| Ok(elapsed_ms() as i32)),
        );
        let instance = tinywasm::ModuleInstance::instantiate(&mut store, &module, Some(imports)).unwrap();
        let func = instance.func::<(), f32>(&store, "run").unwrap();
        func.call(&mut store, ()).unwrap()
    }
}

impl BenchInstance for TinywasmRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
