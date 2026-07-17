#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils::{self as utils, ExecuteTestId};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use fizzy::{Config, Engine, FuncType, Instance, Linker, Module, Val, ValType};

pub struct Fizzy;

impl Runtime for Fizzy {
    fn id(&self) -> &'static str {
        "fizzy"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let config = Config::new();
        let engine = Engine::new(&config);
        Some(Box::new(FizzyInstance {
            linker: Linker::new(&engine),
        }))
    }
}

impl Fizzy {
    fn can_run(&self, id: TestId) -> bool {
        !matches!(
            id,
            TestId::Execute(
                ExecuteTestId::CounterParam | ExecuteTestId::FibonacciTail | ExecuteTestId::BulkOps
            )
        )
    }
}

pub struct FizzyInstance {
    linker: Linker,
}

impl RuntimeInstance for FizzyInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: benchmark_utils::FuncType,
        func: fn(params: &[benchmark_utils::Val], results: &mut [benchmark_utils::Val]),
    ) {
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        let ty = FuncType::new(
            ty.params().iter().copied().map(from_utils_valtype),
            ty.results().iter().copied().map(from_utils_valtype),
        );
        let trampoline = move |params: &[Val], results: &mut [Val]| {
            let in_params: Vec<utils::Val> = params.iter().cloned().map(into_utils_val).collect();
            let mut out: Vec<utils::Val> = result_tys
                .iter()
                .copied()
                .map(utils::Val::default_for_ty)
                .collect();
            func(&in_params, &mut out);
            for (dst, src) in results.iter_mut().zip(out) {
                *dst = from_utils_val(src);
            }
        };
        self.linker.func_new(module, name, ty, trampoline).unwrap();
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let module = Module::new(wasm).unwrap();
        let instance = self.linker.instantiate(&module).unwrap();
        Box::new(FizzyModule {
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

struct FizzyModule {
    instance: Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl ModuleInstance for FizzyModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(name) else {
            bail!("cannot find function: {name}")
        };
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
        self.results.clear();
        self.results
            .extend(func.ty().results().iter().copied().map(Val::default_for_ty));
        func.call(&mut self.instance, &self.params[..], &mut self.results[..])?;
        for (dst, src) in results.iter_mut().zip(&self.results) {
            *dst = into_utils_val(*src);
        }
        Ok(())
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(name) else {
            bail!("cannot find memory: {name}")
        };
        memory.read(&self.instance, ptr, buffer)?;
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(name) else {
            bail!("cannot find memory: {name}")
        };
        memory.write(&mut self.instance, ptr, buffer)?;
        Ok(())
    }
}

fn from_utils_valtype(ty: utils::ValType) -> ValType {
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
