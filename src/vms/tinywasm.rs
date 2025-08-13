use benchmark_utils::{BenchInstance, BenchRuntime, ExecuteTestFilter, TestFilter, elapsed_ms};
use wasmi_new::ModuleImportsIter;

pub struct Tinywasm;

struct TinywasmRuntime {
    store: tinywasm::Store,
    _instance: tinywasm::ModuleInstance,
    func: tinywasm::FuncHandleTyped<i64, i64>,
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
        tinywasm::Module::parse_bytes(wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = tinywasm::Store::new();
        let module = tinywasm::Module::parse_bytes(wasm).unwrap();
        let instance = module.instantiate(&mut store, None).unwrap();
        let func = instance.exported_func::<i64, i64>(&store, "run").unwrap();
        Box::new(TinywasmRuntime {
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = tinywasm::Store::new();
        let module = tinywasm::Module::parse_bytes(wasm).unwrap();
        let mut imports = tinywasm::Imports::new();
        imports
            .define(
                "env",
                "clock_ms",
                tinywasm::Extern::func(
                    &tinywasm::types::FuncType {
                        params: Box::from([]),
                        results: Box::from([tinywasm::types::ValType::I32]),
                    },
                    |_ctx, _args| Ok(vec![tinywasm::types::WasmValue::I32(elapsed_ms() as i32)]),
                ),
            )
            .unwrap();
        let instance = module.instantiate(&mut store, Some(imports)).unwrap();
        let func = instance.exported_func::<(), f32>(&store, "run").unwrap();
        func.call(&mut store, ()).unwrap()
    }
}

impl BenchInstance for TinywasmRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
