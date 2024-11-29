use super::{elapsed_ms, BenchRuntime, BenchVm};
use wasmi_new::ModuleImportsIter;

pub struct WasmiOld;

struct WasmiOldRuntime {
    store: wasmi_old::Store<()>,
    _instance: wasmi_old::Instance,
    func: wasmi_old::TypedFunc<i64, i64>,
}

impl BenchVm for WasmiOld {
    fn name(&self) -> &'static str {
        "wasmi-old"
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        let store = self.store();
        wasmi_old::Module::new(store.engine(), wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut store = self.store();
        let engine = store.engine();
        let module = wasmi_old::Module::new(engine, wasm).unwrap();
        let mut linker = wasmi_old::Linker::new(engine);

        linker
            .func_wrap(
                "fluentbase_v1preview",
                "_write",
                |offset: u32, length: u32| {},
            )
            .unwrap();
        linker
            .func_wrap("fluentbase_v1preview", "_exit", |exit_code: i32| {})
            .unwrap();

        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        let func = instance.get_typed_func::<i64, i64>(&store, "run").unwrap();
        Box::new(WasmiOldRuntime {
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let engine = wasmi_old::Engine::default();
        let mut store = <wasmi_old::Store<()>>::new(&engine, ());
        let mut linker = wasmi_old::Linker::new(store.engine());
        linker
            .func_wrap("env", "clock_ms", || elapsed_ms() as i32)
            .unwrap();

        let module = wasmi_old::Module::new(store.engine(), wasm).unwrap();
        let result = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .ensure_no_start(&mut store)
            .unwrap()
            .get_typed_func::<(), wasmi_old::core::F32>(&mut store, "run")
            .unwrap()
            .call(&mut store, ())
            .unwrap();
        result.into()
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
