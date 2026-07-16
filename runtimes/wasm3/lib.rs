#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils::{self as utils, ExecuteTestId};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
pub use wasm3::CompilationMode;
use wasm3::{Func, Val};

pub struct Wasm3 {
    pub compilation_mode: CompilationMode,
}

/// A concrete Wasm3 runtime with its linker, produced by [`Wasm3::setup`].
struct Wasm3Instance {
    linker: wasm3::Linker<()>,
}

/// An instantiated Wasm3 module, produced by [`Wasm3Instance::instantiate`].
struct Wasm3Module {
    store: wasm3::Store<()>,
    instance: wasm3::Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for Wasm3 {
    fn id(&self) -> &'static str {
        match self.compilation_mode {
            CompilationMode::Eager => "wasm3.eager",
            CompilationMode::Lazy => "wasm3.lazy",
        }
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let linker = wasm3::Linker::new(&make_engine(self.compilation_mode));
        Some(Box::new(Wasm3Instance { linker }))
    }
}

impl Wasm3 {
    fn can_run(&self, id: TestId) -> bool {
        match id {
            TestId::Execute(_) => {
                !matches!(id, TestId::Execute(ExecuteTestId::FibonacciTail))
                    && matches!(self.compilation_mode, CompilationMode::Eager)
            }
            _ => true,
        }
    }
}

impl RuntimeInstance for Wasm3Instance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        let ty = wasm3::FuncType::new(
            ty.params().iter().copied().map(to_wasm3_valtype),
            ty.results().iter().copied().map(to_wasm3_valtype),
        );
        self.linker
            .func_new(
                module,
                name,
                ty,
                move |_caller, params: &[Val], results: &mut [Val]| {
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
                },
            )
            .unwrap();
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let engine = self.linker.engine().clone();
        let mut store = <wasm3::Store<()>>::new(&engine, ());
        let module = wasm3::Module::new(&engine, wasm).unwrap();
        let instance = self
            .linker
            .instantiate_and_start(&mut store, &module)
            .unwrap();
        Box::new(Wasm3Module {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

fn make_engine(compilation_mode: CompilationMode) -> wasm3::Engine {
    let mut config = wasm3::Config::default();
    config.compilation_mode(compilation_mode);
    wasm3::Engine::new(&config)
}

impl ModuleInstance for Wasm3Module {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(&self.store, name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.ty(&self.store)?.params().len());
        assert_eq!(results.len(), func.ty(&self.store)?.results().len());
        self.prepare_params(params);
        self.prepare_results(&func)?;
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(&self.store) else {
            bail!("memory not found: {name}")
        };
        let data = memory.data(&self.store);
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
        let Some(memory) = self.instance.get_memory(&self.store) else {
            bail!("memory not found: {name}")
        };
        let data = memory.data_mut(&mut self.store);
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

impl Wasm3Module {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) -> anyhow::Result<()> {
        self.results.clear();
        for ty in func.ty(&self.store)?.results() {
            self.results.push(Val::default_for_ty(*ty))
        }
        Ok(())
    }

    fn write_back_results(&mut self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            let ty = self.results[i].ty();
            let src = core::mem::replace(&mut self.results[i], Val::default_for_ty(ty));
            *result = into_utils_val(src);
        }
    }
}

fn to_wasm3_valtype(ty: utils::ValType) -> wasm3::ValType {
    match ty {
        utils::ValType::I32 => wasm3::ValType::I32,
        utils::ValType::I64 => wasm3::ValType::I64,
        utils::ValType::F32 => wasm3::ValType::F32,
        utils::ValType::F64 => wasm3::ValType::F64,
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
