#![crate_type = "dylib"]

use benchmark_utils as utils;
use benchmark_utils::{
    CompileTestId, ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, TestId,
};
use makepad_stitch::{Engine, ExternVal, Func, Instance, Linker, Module, Store, Val, ValType};

pub struct Stitch;

struct StitchInstance {
    linker: utils::Linker,
}

struct StitchModule {
    store: Store,
    instance: Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for Stitch {
    fn id(&self) -> &'static str {
        "stitch"
    }

    fn compile(&self, id: CompileTestId, wasm: &[u8]) -> bool {
        if !self.can_run(id.into()) {
            return false;
        }
        let engine = Engine::new();
        Module::new(&engine, wasm).unwrap();
        true
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        Some(Box::new(StitchInstance {
            linker: utils::Linker::new(),
        }))
    }
}

impl Stitch {
    fn can_run(&self, id: TestId) -> bool {
        match id {
            TestId::Compile(id) => !matches!(id, CompileTestId::Ffmpeg),
            TestId::Execute(id) => {
                !matches!(id, ExecuteTestId::FibonacciTail | ExecuteTestId::Argon2)
            }
        }
    }
}

impl RuntimeInstance for StitchInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        self.linker.define(module, name, ty, func);
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let mut store = Store::new(Engine::new());
        let mut linker = Linker::new();
        for (module, name, ty, func) in self.linker.funcs() {
            // Stitch only exposes the typed `Func::wrap` constructor (no untyped/dynamic host
            // function API), so the runtime-neutral signature is matched against the concrete Rust
            // closure types it needs. The benchmark suite only links `env.clock_ms: () -> i32`
            // (for Coremark), so only that signature is supported here. Host functions are bound to
            // a `Store`, so they are (re)built here against the fresh store on every instantiation.
            let host = match (ty.params(), ty.results()) {
                ([], [utils::ValType::I32]) => Func::wrap(&mut store, move || -> i32 {
                    let mut out = [utils::Val::I32(0)];
                    func(&[], &mut out);
                    out[0].unwrap_i32()
                }),
                _ => unimplemented!(
                    "the stitch adapter only supports the `() -> i32` host function signature used by Coremark"
                ),
            };
            linker.define(module, name, ExternVal::Func(host));
        }
        let module = Module::new(store.engine(), wasm).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        Box::new(StitchModule {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

impl ModuleInstance for StitchModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.exported_func(name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.type_(&self.store).params().len());
        assert_eq!(results.len(), func.type_(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }
}

impl StitchModule {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) {
        self.results.clear();
        for ty in func.type_(&self.store).results() {
            self.results.push(default_val(*ty))
        }
    }

    fn write_back_results(&self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            *result = into_utils_val(self.results[i]);
        }
    }
}

fn default_val(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0.0),
        ValType::F64 => Val::F64(0.0),
        _ => unreachable!(),
    }
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(val) => Val::I32(val),
        utils::Val::I64(val) => Val::I64(val),
        utils::Val::F32(val) => Val::F32(val),
        utils::Val::F64(val) => Val::F64(val),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(val) => utils::Val::I32(val),
        Val::I64(val) => utils::Val::I64(val),
        Val::F32(val) => utils::Val::F32(val),
        Val::F64(val) => utils::Val::F64(val),
        _ => panic!(),
    }
}
