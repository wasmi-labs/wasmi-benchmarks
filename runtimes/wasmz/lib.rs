#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils as utils;
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use wasmz::{Engine, Instance, Linker, Module, Store, Val, ValKind};

pub struct Wasmz;

struct WasmzInstance {
    engine: Engine,
    linker: Linker,
}

/// Fields drop in declaration order, so the instance must come first: it holds
/// borrowed handles into the store and the module.
struct WasmzModule {
    instance: Instance,
    #[allow(dead_code, reason = "kept alive for instance lifetime")]
    module: Module,
    #[allow(dead_code, reason = "kept alive for instance lifetime")]
    store: Store,
}

impl Runtime for Wasmz {
    fn id(&self) -> &'static str {
        "wasmz"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let engine = Engine::new().expect("wasmz engine");
        let linker = Linker::new().expect("wasmz linker");
        Some(Box::new(WasmzInstance { engine, linker }))
    }
}

impl Wasmz {
    fn can_run(&self, _id: TestId) -> bool {
        true
    }
}

impl RuntimeInstance for WasmzInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let params: Vec<ValKind> = ty.params().iter().copied().map(to_wasmz_valkind).collect();
        let results: Vec<ValKind> = ty.results().iter().copied().map(to_wasmz_valkind).collect();
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        self.linker
            .define_func(
                module,
                name,
                &params,
                &results,
                Box::new(move |params, results| {
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
                }),
            )
            .expect("wasmz link_func");
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let store = Store::new(&self.engine).expect("wasmz store");
        let module = Module::compile(&self.engine, wasm).expect("wasmz compile");
        let instance =
            Instance::new(&store, &module, Some(&self.linker)).expect("wasmz instantiate");
        Box::new(WasmzModule {
            instance,
            module,
            store,
        })
    }
}

impl ModuleInstance for WasmzModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let args: Vec<Val> = params.iter().copied().map(from_utils_val).collect();
        let result_kinds: Vec<ValKind> = results
            .iter()
            .map(|val| to_wasmz_valkind(val.ty()))
            .collect();
        let out = self
            .instance
            .call(name, &args, &result_kinds)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        for (dst, src) in results.iter_mut().zip(out) {
            *dst = into_utils_val(src);
        }
        Ok(())
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let _ = name;
        let Some((mem, size)) = self.instance.memory() else {
            bail!("memory not found")
        };
        let ptr = ptr as usize;
        let end = ptr + buffer.len();
        if end > size {
            bail!("memory read out of bounds");
        }
        unsafe {
            std::ptr::copy_nonoverlapping(mem.add(ptr), buffer.as_mut_ptr(), buffer.len());
        }
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let _ = name;
        let Some((mem, size)) = self.instance.memory() else {
            bail!("memory not found")
        };
        let ptr = ptr as usize;
        let end = ptr + buffer.len();
        if end > size {
            bail!("memory write out of bounds");
        }
        unsafe {
            std::ptr::copy_nonoverlapping(buffer.as_ptr(), mem.add(ptr), buffer.len());
        }
        Ok(())
    }
}

fn to_wasmz_valkind(ty: utils::ValType) -> ValKind {
    match ty {
        utils::ValType::I32 => ValKind::I32,
        utils::ValType::I64 => ValKind::I64,
        utils::ValType::F32 => ValKind::F32,
        utils::ValType::F64 => ValKind::F64,
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
