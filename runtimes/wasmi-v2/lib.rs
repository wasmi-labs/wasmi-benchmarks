#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils::{self as utils};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
pub use wasmi::CompilationMode;
use wasmi::{Func, Val};

pub struct Wasmi {
    pub compilation_mode: CompilationMode,
    pub validation: Validation,
}

#[derive(Debug, Copy, Clone)]
pub enum Validation {
    Checked,
    Unchecked,
}

/// A concrete Wasmi runtime with its linker, produced by [`Wasmi::setup`].
struct WasmiInstance {
    linker: wasmi::Linker<()>,
    validation: Validation,
}

/// An instantiated Wasmi module, produced by [`WasmiInstance::instantiate`].
struct WasmiModule {
    store: wasmi::Store<()>,
    instance: wasmi::Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for Wasmi {
    fn id(&self) -> &'static str {
        match (self.compilation_mode, self.validation) {
            (CompilationMode::Eager, Validation::Checked) => "wasmi-v2.eager.checked",
            (CompilationMode::Eager, Validation::Unchecked) => "wasmi-v2.eager.unchecked",
            (CompilationMode::LazyTranslation, Validation::Checked) => {
                "wasmi-v2.lazy-translation.checked"
            }
            (CompilationMode::LazyTranslation, Validation::Unchecked) => {
                "wasmi-v2.lazy-translation.unchecked"
            }
            (CompilationMode::Lazy, Validation::Checked) => "wasmi-v2.lazy.checked",
            (CompilationMode::Lazy, Validation::Unchecked) => "wasmi-v2.lazy.unchecked",
        }
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let linker = wasmi::Linker::new(&make_engine(self.compilation_mode));
        Some(Box::new(WasmiInstance {
            linker,
            validation: self.validation,
        }))
    }
}

impl Wasmi {
    fn can_run(&self, id: TestId) -> bool {
        match id {
            TestId::Execute(_) => {
                matches!(self.validation, Validation::Checked)
                    && matches!(self.compilation_mode, CompilationMode::Eager)
            }
            _ => true,
        }
    }
}

impl RuntimeInstance for WasmiInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        let ty = wasmi::FuncType::new(
            ty.params().iter().copied().map(to_wasmi_valtype),
            ty.results().iter().copied().map(to_wasmi_valtype),
        );
        self.linker
            .func_new(
                module,
                name,
                ty,
                move |_caller, params: &[Val], results: &mut [Val]| {
                    let in_params: Vec<utils::Val> =
                        params.iter().cloned().map(into_utils_val).collect();
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
        let mut store = <wasmi::Store<()>>::new(&engine, ());
        let module = make_module(self.validation, &engine, wasm);
        let instance = self
            .linker
            .instantiate_and_start(&mut store, &module)
            .unwrap();
        Box::new(WasmiModule {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

fn make_engine(compilation_mode: CompilationMode) -> wasmi::Engine {
    let mut config = wasmi::Config::default();
    config.wasm_tail_call(true);
    config.compilation_mode(compilation_mode);
    wasmi::Engine::new(&config)
}

fn make_module(validation: Validation, engine: &wasmi::Engine, wasm: &[u8]) -> wasmi::Module {
    match validation {
        Validation::Checked => wasmi::Module::new(engine, wasm).unwrap(),
        Validation::Unchecked => {
            // SAFETY: We only use properly valid Wasm in our benchmarks.
            unsafe { wasmi::Module::new_unchecked(engine, wasm).unwrap() }
        }
    }
}

impl ModuleInstance for WasmiModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(&self.store, name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.ty(&self.store).params().len());
        assert_eq!(results.len(), func.ty(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }

    fn read_memory(&self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(&self.store, name) else {
            bail!("memory not found: {name}")
        };
        memory.read(&self.store, ptr as usize, buffer)?;
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(&self.store, name) else {
            bail!("memory not found: {name}")
        };
        memory.write(&mut self.store, ptr as usize, buffer)?;
        Ok(())
    }
}

impl WasmiModule {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) {
        self.results.clear();
        for ty in func.ty(&self.store).results() {
            self.results.push(Val::default_for_ty(*ty))
        }
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

fn to_wasmi_valtype(ty: utils::ValType) -> wasmi::ValType {
    match ty {
        utils::ValType::I32 => wasmi::ValType::I32,
        utils::ValType::I64 => wasmi::ValType::I64,
        utils::ValType::F32 => wasmi::ValType::F32,
        utils::ValType::F64 => wasmi::ValType::F64,
    }
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(val) => Val::I32(val),
        utils::Val::I64(val) => Val::I64(val),
        utils::Val::F32(val) => Val::F32(val.into()),
        utils::Val::F64(val) => Val::F64(val.into()),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(val) => utils::Val::I32(val),
        Val::I64(val) => utils::Val::I64(val),
        Val::F32(val) => utils::Val::F32(val.to_float()),
        Val::F64(val) => utils::Val::F64(val.to_float()),
        _ => panic!(),
    }
}
