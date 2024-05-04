use super::{BenchRuntime, BenchVm};

pub struct WasmiOld;

struct WasmiOldRuntime {
    store: wasmi_old::Store<()>,
    instance: wasmi_old::Instance,
    func: wasmi_old::TypedFunc<i64, i64>,
}

impl BenchVm for WasmiOld {
    fn name(&self) -> &'static str {
        "wasmi-v0.31"
    }

    fn compile(&self, wasm: &[u8]) {
        let mut store = self.store();
        wasmi_old::Module::new(store.engine(), &wasm[..]).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut store = self.store();
        let engine = store.engine();
        let module = wasmi_old::Module::new(engine, &wasm[..]).unwrap();
        let linker = wasmi_old::Linker::new(engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Box::new(WasmiOldRuntime {
            store,
            instance,
            func,
        })
    }
}

impl WasmiOld {
    fn store(&self) -> wasmi_old::Store<()> {
        let mut config = wasmi_old::Config::default();
        config.wasm_tail_call(true);
        let engine = wasmi_old::Engine::new(&config);
        <wasmi_old::Store<()>>::new(&engine, ())
    }
}

impl BenchRuntime for WasmiOldRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
