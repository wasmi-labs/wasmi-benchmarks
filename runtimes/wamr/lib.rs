#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils::{self as utils};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use wamr::{Engine, Func, FuncType, Instance, Linker, Module, Val, ValType};

pub struct Wamr;

struct WamrInstance {
    engine: Engine,
    linker: Linker,
}

struct WamrModule {
    instance: Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for Wamr {
    fn id(&self) -> &'static str {
        "wamr"
    }

    fn setup(&self, _id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        let engine = Engine::new().unwrap();
        let linker = Linker::new(&engine);
        Some(Box::new(WamrInstance { engine, linker }))
    }
}

impl RuntimeInstance for WamrInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: benchmark_utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let ty = FuncType::new(
            ty.params().iter().copied().map(to_wamr_valtype),
            ty.results().iter().copied().map(to_wamr_valtype),
        );
        let trampoline = move |params: &[Val], results: &mut [Val]| {
            let utils_params: Vec<utils::Val> =
                params.iter().copied().map(into_utils_val).collect();
            let mut utils_results: Vec<utils::Val> =
                results.iter().copied().map(into_utils_val).collect();
            func(&utils_params[..], &mut utils_results[..]);
            for (dst, src) in results.iter_mut().zip(utils_results) {
                *dst = from_utils_val(src);
            }
        };
        self.linker
            .define_func(module, name, ty, trampoline)
            .unwrap();
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let engine = self.engine.clone();
        let module = Module::new(&engine, wasm).unwrap();
        let instance = self.linker.instantiate(module).unwrap();
        Box::new(WamrModule {
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

impl ModuleInstance for WamrModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let func = self.instance.get_func(name)?;
        assert_eq!(params.len(), func.ty().params().len());
        assert_eq!(results.len(), func.ty().results().len());
        Self::prepare_params(&mut self.params, params);
        Self::prepare_results(&mut self.results, &func);
        func.call(&self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }

    fn read_memory(&self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
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

impl WamrModule {
    fn prepare_params(dst: &mut Vec<Val>, src: &[utils::Val]) {
        dst.clear();
        dst.extend(src.iter().copied().map(from_utils_val));
    }

    fn prepare_results(dst: &mut Vec<Val>, func: &Func) {
        dst.clear();
        for ty in func.ty().results() {
            dst.push(default_for_ty(*ty))
        }
    }

    fn write_back_results(&mut self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            let ty = self.results[i].ty();
            let src = core::mem::replace(&mut self.results[i], default_for_ty(ty));
            *result = into_utils_val(src);
        }
    }
}

fn default_for_ty(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0.0),
        ValType::F64 => Val::F64(0.0),
    }
}

fn to_wamr_valtype(ty: utils::ValType) -> ValType {
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
