#![crate_type = "dylib"]

use benchmark_utils::{self as utils, CompileTestId};
use benchmark_utils::{ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, TestId};
use tinywasm::types::{FuncType as TinyFuncType, WasmType, WasmValue as Val};

pub struct Tinywasm;

/// A concrete Tinywasm runtime with its store and imports, produced by [`Tinywasm::setup`].
///
/// Tinywasm binds host functions to a [`Store`](tinywasm::Store), so the store is created up
/// front and the module that [`RuntimeInstance::load`] instantiates runs in this same store.
struct TinywasmInstance {
    store: tinywasm::Store,
    imports: tinywasm::Imports,
}

/// An instantiated Tinywasm module, produced by [`TinywasmInstance::load`].
struct TinywasmModule {
    store: tinywasm::Store,
    instance: tinywasm::ModuleInstance,
    params: Vec<Val>,
}

impl Runtime for Tinywasm {
    fn id(&self) -> &'static str {
        "tinywasm"
    }

    fn compile(&self, id: CompileTestId, wasm: &[u8]) -> bool {
        if !self.can_run(id.into()) {
            return false;
        }
        tinywasm::parse_bytes(wasm).unwrap();
        true
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        Some(Box::new(TinywasmInstance {
            store: tinywasm::Store::default(),
            imports: tinywasm::Imports::new(),
        }))
    }
}

impl Tinywasm {
    fn can_run(&self, id: TestId) -> bool {
        !matches!(id, TestId::Execute(ExecuteTestId::FibonacciTail))
    }
}

impl RuntimeInstance for TinywasmInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        let params: Vec<WasmType> = ty.params().iter().copied().map(to_wasm_type).collect();
        let results: Vec<WasmType> = ty.results().iter().copied().map(to_wasm_type).collect();
        let ty = TinyFuncType::new(&params, &results);
        let host = tinywasm::HostFunction::from_untyped(
            &mut self.store,
            &ty,
            move |_ctx, args: &[Val]| {
                let in_params: Vec<utils::Val> = args.iter().copied().map(into_utils_val).collect();
                let mut out: Vec<utils::Val> = result_tys
                    .iter()
                    .copied()
                    .map(utils::Val::default_for_ty)
                    .collect();
                func(&in_params, &mut out);
                Ok(out.into_iter().map(from_utils_val).collect())
            },
        );
        self.imports.define(module, name, host);
    }

    fn instantiate(self: Box<Self>, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let TinywasmInstance { mut store, imports } = *self;
        let module = tinywasm::parse_bytes(wasm).unwrap();
        let instance =
            tinywasm::ModuleInstance::instantiate(&mut store, &module, Some(imports)).unwrap();
        Box::new(TinywasmModule {
            store,
            instance,
            params: Vec::new(),
        })
    }
}

impl ModuleInstance for TinywasmModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let func = self.instance.func_untyped(&self.store, name)?;
        self.prepare_params(params);
        let call_results = func.call(&mut self.store, &self.params[..])?;
        self.write_back_results(results, &call_results[..]);
        Ok(())
    }
}

impl TinywasmModule {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn write_back_results(&self, dst: &mut [utils::Val], src: &[Val]) {
        assert_eq!(dst.len(), src.len());
        for (i, result) in dst.iter_mut().enumerate() {
            *result = into_utils_val(src[i]);
        }
    }
}

fn to_wasm_type(ty: utils::ValType) -> WasmType {
    match ty {
        utils::ValType::I32 => WasmType::I32,
        utils::ValType::I64 => WasmType::I64,
        utils::ValType::F32 => WasmType::F32,
        utils::ValType::F64 => WasmType::F64,
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
