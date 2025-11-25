#![crate_type = "dylib"]

use benchmark_utils::{BenchInstance, BenchRuntime, ModuleImportsIter, elapsed_ms};

pub struct Wasmi032;

struct Wasmi032Runtime {
    store: wasmi032::Store<()>,
    _instance: wasmi032::Instance,
    func: wasmi032::TypedFunc<i64, i64>,
}

impl BenchRuntime for Wasmi032 {
    fn name(&self) -> &'static str {
        "wasmi-v0.32"
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        let store = self.store();
        wasmi032::Module::new(store.engine(), wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = self.store();
        let engine = store.engine();
        let module = wasmi032::Module::new(engine, wasm).unwrap();
        let linker = wasmi032::Linker::new(engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Box::new(Wasmi032Runtime {
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let engine = wasmi032::Engine::default();
        let mut store = <wasmi032::Store<()>>::new(&engine, ());
        let mut linker = wasmi032::Linker::new(store.engine());
        linker
            .func_wrap("env", "clock_ms", || elapsed_ms() as i32)
            .unwrap();
        let module = wasmi032::Module::new(store.engine(), wasm).unwrap();
        let result = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .ensure_no_start(&mut store)
            .unwrap()
            .get_typed_func::<(), wasmi032::core::F32>(&mut store, "run")
            .unwrap()
            .call(&mut store, ())
            .unwrap();
        result.into()
    }
}

impl Wasmi032 {
    fn store(&self) -> wasmi032::Store<()> {
        let mut config = wasmi032::Config::default();
        config.wasm_tail_call(true);
        let engine = wasmi032::Engine::new(&config);
        <wasmi032::Store<()>>::new(&engine, ())
    }
}

impl BenchInstance for Wasmi032Runtime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
