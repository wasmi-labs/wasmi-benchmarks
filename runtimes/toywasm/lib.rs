#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils::{self as utils};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use toywasmx::{FuncType, Store, Val, ValType};

pub struct Toywasm;

/// A concrete Toywasm runtime with its recorded host functions, produced by [`Toywasm::setup`].
///
/// Toywasm binds host functions to a [`Store`] via a [`Linker`](toywasmx::Linker), so rather than
/// holding a live store the recorded host functions are replayed into a fresh store on every
/// instantiation.
struct ToywasmInstance {
    linker: utils::Linker,
}

/// An instantiated Toywasm module, produced by [`ToywasmInstance::instantiate`].
///
/// The [`Instance`](toywasmx::Instance) keeps the store's memory context alive, so it is the only
/// state the module wrapper needs to hold.
struct ToywasmModule {
    instance: toywasmx::Instance,
}

impl Runtime for Toywasm {
    fn id(&self) -> &'static str {
        "toywasm"
    }

    fn setup(&self, _id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        Some(Box::new(ToywasmInstance {
            linker: utils::Linker::new(),
        }))
    }
}

impl RuntimeInstance for ToywasmInstance {
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
        // Toywasm binds host functions to a `Store` through a `Linker`, so the recorded functions
        // are (re)built against a fresh store and linker on every instantiation.
        let store = Store::new();
        let mut linker = toywasmx::Linker::new();
        for (module, name, ty, func) in self.linker.funcs() {
            let result_tys: Vec<utils::ValType> = ty.results().to_vec();
            let params: Vec<ValType> = ty.params().iter().copied().map(to_ty).collect();
            let results: Vec<ValType> = ty.results().iter().copied().map(to_ty).collect();
            let ty = FuncType::new(params, results);
            linker.func_new(module, name, &ty, move |args: &[Val], out: &mut [Val]| {
                let in_params: Vec<utils::Val> = args.iter().copied().map(into_utils_val).collect();
                let mut host_out: Vec<utils::Val> = result_tys
                    .iter()
                    .copied()
                    .map(utils::Val::default_for_ty)
                    .collect();
                func(&in_params, &mut host_out);
                for (dst, val) in out.iter_mut().zip(host_out) {
                    *dst = from_utils_val(val);
                }
            });
        }
        let module = toywasmx::Module::new(&store, wasm).unwrap();
        let instance = linker.instantiate(&store, module).unwrap();
        Box::new(ToywasmModule { instance })
    }
}

impl ModuleInstance for ToywasmModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let func = self.instance.get_func(name)?;
        let call_params: Vec<Val> = params.iter().copied().map(from_utils_val).collect();
        let mut call_results: Vec<Val> = func
            .ty()
            .results()
            .iter()
            .copied()
            .map(Val::default_for_ty)
            .collect();
        func.call(&mut self.instance, &call_params, &mut call_results)?;
        assert_eq!(results.len(), call_results.len());
        for (dst, val) in results.iter_mut().zip(call_results) {
            *dst = into_utils_val(val);
        }
        Ok(())
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let memory = self.instance.get_memory(name)?;
        let data = memory.data();
        let ptr = ptr as usize;
        let len = buffer.len();
        let Some(bytes) = data.get(ptr..ptr + len) else {
            bail!(
                "failed to slice bytes from {name} at {ptr} with length {}",
                buffer.len()
            )
        };
        buffer.copy_from_slice(bytes);
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let mut memory = self.instance.get_memory(name)?;
        let data = memory.data_mut();
        let ptr = ptr as usize;
        let len = buffer.len();
        let Some(bytes) = data.get_mut(ptr..ptr + len) else {
            bail!(
                "failed to slice bytes from {name} at {ptr} with length {}",
                buffer.len()
            )
        };
        bytes.copy_from_slice(buffer);
        Ok(())
    }
}

fn to_ty(ty: utils::ValType) -> ValType {
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
    }
}
