use super::{BenchRuntime, BenchVm};
use crate::utils::TestFilter;

pub struct Wasmtime {
    pub strategy: wasmtime::Strategy,
}

pub struct WasmtimeRuntime {
    store: wasmtime::Store<()>,
    instance: wasmtime::Instance,
    func: wasmtime::TypedFunc<i64, i64>,
}

impl BenchVm for Wasmtime {
    fn name(&self) -> &'static str {
        match self.strategy {
            wasmtime::Strategy::Cranelift | wasmtime::Strategy::Auto => "wasmtime.cranelift",
            wasmtime::Strategy::Winch => "wasmtime.winch",
            _ => panic!("unknown Wasmtime strategy"),
        }
    }

    fn test_filter(&self) -> TestFilter {
        match self.strategy {
            wasmtime::Strategy::Auto | wasmtime::Strategy::Cranelift => TestFilter::default(),
            wasmtime::Strategy::Winch => {
                let winch_works = cfg!(target_arch = "x86_64");
                TestFilter {
                    fib_tailrec: false,
                    ..TestFilter::set_to(winch_works)
                }
            }
            unknown => panic!("unknown Wasmtime strategy: {unknown:?}"),
        }
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let mut config = wasmtime::Config::default();
        config.wasm_tail_call(true);
        config.strategy(self.strategy);
        let engine = wasmtime::Engine::new(&config).unwrap();
        let mut store = <wasmtime::Store<()>>::new(&engine, ());
        let module = wasmtime::Module::new(&engine, &wasm[..]).unwrap();
        let linker = wasmtime::Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let func = instance
            .get_typed_func::<i64, i64>(&mut store, "run")
            .unwrap();
        Box::new(WasmtimeRuntime {
            store,
            instance,
            func,
        })
    }
}

impl BenchRuntime for WasmtimeRuntime {
    fn call(&mut self, input: i64) {
        self.func.call(&mut self.store, input).unwrap();
    }
}
