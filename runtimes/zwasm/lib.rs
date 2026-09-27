#![crate_type = "dylib"]
#![cfg(any(feature = "jit", feature = "interp"))]

// Relies on two properties of this harness: modules import functions only (zwasm cannot import a
// memory, zwasm-rust-sdk#22) and everything runs on one thread (zwasm's engine is single-threaded
// per process, zwasm-rust-sdk#33).

use anyhow::{anyhow, bail};
use benchmark_utils as utils;
use benchmark_utils::{
    ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, StartupTestId, TestId,
};
use zwasm_sdk::{Engine, EngineKind, ExternKind, Func, Instance, Module, Store, Val, ValType};

#[derive(Debug, Copy, Clone)]
pub enum Tier {
    Jit,
    Interp,
}

impl Tier {
    fn id(self) -> &'static str {
        match self {
            Tier::Jit => "zwasm.jit",
            Tier::Interp => "zwasm.interp",
        }
    }

    fn engine_kind(self) -> EngineKind {
        match self {
            Tier::Jit => EngineKind::Jit,
            Tier::Interp => EngineKind::Interp,
        }
    }
}

pub struct Zwasm {
    pub tier: Tier,
}

struct ZwasmInstance {
    tier: Tier,
    engine: Engine,
    linker: utils::Linker,
}

/// `Instance` is a `Copy` handle into the store, which owns every C object and frees them on drop.
struct ZwasmModule {
    instance: Instance,
    store: Store,
}

impl Runtime for Zwasm {
    fn id(&self) -> &'static str {
        self.tier.id()
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let engine = Engine::new().expect("zwasm engine");
        Some(Box::new(ZwasmInstance {
            tier: self.tier,
            engine,
            linker: utils::Linker::new(),
        }))
    }
}

impl Zwasm {
    fn can_run(&self, id: TestId) -> bool {
        match self.tier {
            Tier::Jit => match id {
                // Note: zwasm's JIT aborts while compiling ffmpeg: register allocator overflow,
                // zwasm/zwasm#487.
                TestId::Startup(StartupTestId::Ffmpeg) => false,
                _ => {
                    // Note: zwasm's JIT only targets `x86_64` and `aarch64`.
                    cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64")
                }
            },
            Tier::Interp => match id {
                // Note: zwasm's interpreter traps on a `loop` with block parameters
                // (zwasm/zwasm#244).
                TestId::Execute(ExecuteTestId::CounterParam) => false,
                // Note: zwasm's interpreter rejects a module importing `wasi_snapshot_preview1`
                // unless the store has a WASI host, even with the imports provided by the host
                // (zwasm/zwasm#488).
                TestId::Startup(StartupTestId::Bz2)
                | TestId::Startup(StartupTestId::PulldownCmark)
                | TestId::Startup(StartupTestId::Spidermonkey)
                | TestId::Startup(StartupTestId::Ffmpeg) => false,
                _ => true,
            },
        }
    }
}

impl RuntimeInstance for ZwasmInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        self.linker.define(module, name, ty, func);
    }

    // Host functions belong to a store and every instantiation gets a fresh one, so they are
    // created here rather than in `link_func`.
    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let mut store = Store::new(&self.engine).expect("zwasm store");
        let module = Module::new(&mut store, wasm).expect("zwasm module");
        let mut imports: Vec<Func> = Vec::new();
        for import in module.imports(&store) {
            let (module_name, name) = (import.module(), import.name());
            assert!(
                matches!(import.kind(), ExternKind::Func),
                "unsupported non-function import {module_name}::{name}"
            );
            let (ty, func) = self
                .linker
                .funcs()
                .find(|(module, name_, _, _)| *module == module_name && *name_ == name)
                .map(|(_, _, ty, func)| (ty, func))
                .unwrap_or_else(|| panic!("unlinked import {module_name}::{name}"));
            let params: Vec<ValType> = ty.params().iter().copied().map(to_zwasm_valtype).collect();
            let results: Vec<ValType> =
                ty.results().iter().copied().map(to_zwasm_valtype).collect();
            let result_tys: Vec<utils::ValType> = ty.results().to_vec();
            let host = Func::new(&mut store, &params, &results, move |params, results| {
                let in_params: Vec<utils::Val> =
                    params.iter().copied().map(into_utils_val).collect();
                let mut out: Vec<utils::Val> = result_tys
                    .iter()
                    .copied()
                    .map(utils::Val::default_for_ty)
                    .collect();
                func(&in_params, &mut out);
                for (dst, src) in results.iter_mut().zip(out) {
                    *dst = from_utils_val(src);
                }
                Ok(())
            })
            .expect("zwasm host func");
            imports.push(host);
        }
        let instance =
            Instance::new_with_engine(&mut store, &module, &imports, self.tier.engine_kind())
                .unwrap_or_else(|error| panic!("{} instantiate: {error}", self.tier.id()));
        Box::new(ZwasmModule { instance, store })
    }
}

impl ModuleInstance for ZwasmModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(&mut self.store, name) else {
            bail!("function not found: {name}")
        };
        let args: Vec<Val> = params.iter().copied().map(from_utils_val).collect();
        let mut out: Vec<Val> = results.iter().copied().map(from_utils_val).collect();
        func.call(&mut self.store, &args, &mut out)
            .map_err(|error| anyhow!("{error}"))?;
        for (dst, src) in results.iter_mut().zip(out) {
            *dst = into_utils_val(src);
        }
        Ok(())
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(&mut self.store, name) else {
            bail!("memory not found: {name}")
        };
        let start = ptr as usize;
        let Some(src) = memory.data(&self.store).get(start..start + buffer.len()) else {
            bail!("memory read out of bounds")
        };
        buffer.copy_from_slice(src);
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(&mut self.store, name) else {
            bail!("memory not found: {name}")
        };
        let start = ptr as usize;
        let Some(dst) = memory
            .data_mut(&mut self.store)
            .get_mut(start..start + buffer.len())
        else {
            bail!("memory write out of bounds")
        };
        dst.copy_from_slice(buffer);
        Ok(())
    }
}

fn to_zwasm_valtype(ty: utils::ValType) -> ValType {
    match ty {
        utils::ValType::I32 => ValType::I32,
        utils::ValType::I64 => ValType::I64,
        utils::ValType::F32 => ValType::F32,
        utils::ValType::F64 => ValType::F64,
    }
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(v) => Val::I32(v),
        utils::Val::I64(v) => Val::I64(v),
        utils::Val::F32(v) => Val::F32(v),
        utils::Val::F64(v) => Val::F64(v),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(v) => utils::Val::I32(v),
        Val::I64(v) => utils::Val::I64(v),
        Val::F32(v) => utils::Val::F32(v),
        Val::F64(v) => utils::Val::F64(v),
    }
}
