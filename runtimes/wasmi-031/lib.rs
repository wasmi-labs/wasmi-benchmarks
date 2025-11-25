#![crate_type = "dylib"]

use benchmark_utils::{BenchInstance, BenchRuntime, ModuleImportsIter, elapsed_ms};

pub struct Wasmi031;

struct Wasmi031Runtime {
    store: wasmi031::Store<()>,
    _instance: wasmi031::Instance,
    func: wasmi031::TypedFunc<i64, i64>,
}

impl BenchRuntime for Wasmi031 {
    fn name(&self) -> &'static str {
        "wasmi-v0.31"
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        let store = self.store();
        wasmi031::Module::new(store.engine(), wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = self.store();
        let engine = store.engine();
        let module = wasmi031::Module::new(engine, wasm).unwrap();
        let linker = wasmi031::Linker::new(engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Box::new(Wasmi031Runtime {
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let engine = wasmi031::Engine::default();
        let mut store = <wasmi031::Store<()>>::new(&engine, ());
        let mut linker = wasmi031::Linker::new(store.engine());
        linker
            .func_wrap("env", "clock_ms", || elapsed_ms() as i32)
            .unwrap();
        let module = wasmi031::Module::new(store.engine(), wasm).unwrap();
        let result = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .ensure_no_start(&mut store)
            .unwrap()
            .get_typed_func::<(), wasmi031::core::F32>(&mut store, "run")
            .unwrap()
            .call(&mut store, ())
            .unwrap();
        result.into()
    }
}

impl Wasmi031 {
    fn store(&self) -> wasmi031::Store<()> {
        let mut config = wasmi031::Config::default();
        config.wasm_tail_call(true);
        let engine = wasmi031::Engine::new(&config);
        <wasmi031::Store<()>>::new(&engine, ())
    }
}

impl BenchInstance for Wasmi031Runtime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
