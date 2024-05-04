use super::{BenchRuntime, BenchVm};
use crate::utils::TestFilter;

pub struct Tinywasm;

pub struct TinywasmRuntime {
    store: tinywasm::Store,
    instance: tinywasm::ModuleInstance,
    func: tinywasm::FuncHandleTyped<i64, i64>,
}

impl BenchVm for Tinywasm {
    fn name(&self) -> &'static str {
        "tinywasm"
    }

    fn test_filter(&self) -> TestFilter {
        TestFilter {
            fib_tailrec: false,
            ..Default::default()
        }
    }

    fn compile(&self, wasm: &[u8]) {
        tinywasm::Module::parse_bytes(&wasm[..]).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut store = tinywasm::Store::new();
        let module = tinywasm::Module::parse_bytes(&wasm[..]).unwrap();
        let instance = module.instantiate(&mut store, None).unwrap();
        let func = instance
            .exported_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Box::new(TinywasmRuntime {
            store,
            instance,
            func,
        })
    }
}

impl BenchRuntime for TinywasmRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
