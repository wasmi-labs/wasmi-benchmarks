#![crate_type = "dylib"]

use benchmark_utils::{self as utils, CompileTestId};
use benchmark_utils::{ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, TestId};
use wasmer::Type as ValType;
use wasmer::Value as Val;

#[derive(Debug, Copy, Clone)]
pub enum WasmerCompiler {
    Cranelift,
    Singlepass,
}

pub struct Wasmer {
    pub compiler: WasmerCompiler,
}

/// A concrete Wasmer runtime with its store and imports, produced by [`Wasmer::setup`].
///
/// Wasmer binds host functions to a [`Store`](wasmer::Store), so the store is created up front
/// and the module that [`RuntimeInstance::load`] instantiates runs in this same store.
struct WasmerInstance {
    store: wasmer::Store,
    imports: wasmer::Imports,
}

/// An instantiated Wasmer module, produced by [`WasmerInstance::load`].
struct WasmerModule {
    store: wasmer::Store,
    instance: wasmer::Instance,
    params: Vec<Val>,
}

impl Runtime for Wasmer {
    fn id(&self) -> &'static str {
        match self.compiler {
            WasmerCompiler::Cranelift => "wasmer.cranelift",
            WasmerCompiler::Singlepass => "wasmer.singlepass",
        }
    }

    fn compile(&self, id: CompileTestId, wasm: &[u8]) -> bool {
        if !self.can_run(id.into()) {
            return false;
        }
        let store = make_store(self.compiler);
        wasmer::Module::new(&store, wasm).unwrap();
        true
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        Some(Box::new(WasmerInstance {
            store: make_store(self.compiler),
            imports: wasmer::Imports::new(),
        }))
    }
}

impl Wasmer {
    fn can_run(&self, id: TestId) -> bool {
        !matches!(id, TestId::Execute(ExecuteTestId::FibonacciTail))
    }
}

impl RuntimeInstance for WasmerInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        let params: Vec<ValType> = ty.params().iter().copied().map(to_wasmer_type).collect();
        let results: Vec<ValType> = ty.results().iter().copied().map(to_wasmer_type).collect();
        let ty = wasmer::FunctionType::new(params, results);
        let host = wasmer::Function::new(&mut self.store, ty, move |args: &[Val]| {
            let in_params: Vec<utils::Val> = args.iter().cloned().map(into_utils_val).collect();
            let mut out: Vec<utils::Val> = result_tys
                .iter()
                .copied()
                .map(utils::Val::default_for_ty)
                .collect();
            func(&in_params, &mut out);
            Ok(out.into_iter().map(from_utils_val).collect())
        });
        self.imports.define(module, name, host);
    }

    fn instantiate(self: Box<Self>, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let WasmerInstance { mut store, imports } = *self;
        let module = wasmer::Module::new(&store, wasm).unwrap();
        let instance = wasmer::Instance::new(&mut store, &module, &imports).unwrap();
        Box::new(WasmerModule {
            store,
            instance,
            params: Vec::new(),
        })
    }
}

fn make_store(compiler: WasmerCompiler) -> wasmer::Store {
    match compiler {
        WasmerCompiler::Cranelift => {
            let builder =
                wasmer::sys::EngineBuilder::new(wasmer_compiler_cranelift::Cranelift::new());
            let mut features = wasmer::sys::Features::new();
            features.tail_call(true);
            let engine = builder.set_features(Some(features)).engine();
            wasmer::Store::new(engine)
        }
        WasmerCompiler::Singlepass => {
            let builder =
                wasmer::sys::EngineBuilder::new(wasmer_compiler_singlepass::Singlepass::new());
            wasmer::Store::new(builder)
        }
    }
}

impl ModuleInstance for WasmerModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let func = self.instance.exports.get_function(name).cloned()?;
        assert_eq!(params.len(), func.ty(&self.store).params().len());
        assert_eq!(results.len(), func.ty(&self.store).results().len());
        self.prepare_params(params);
        let call_results = func.call(&mut self.store, &self.params[..])?;
        self.write_back_results(results, &call_results);
        Ok(())
    }
}

impl WasmerModule {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn write_back_results(&self, dst: &mut [utils::Val], src: &[Val]) {
        assert_eq!(dst.len(), src.len());
        for (dst, src) in dst.iter_mut().zip(src) {
            *dst = into_utils_val(src.clone());
        }
    }
}

fn to_wasmer_type(ty: utils::ValType) -> ValType {
    match ty {
        utils::ValType::I32 => ValType::I32,
        utils::ValType::I64 => ValType::I64,
        utils::ValType::F32 => ValType::F32,
        utils::ValType::F64 => ValType::F64,
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
