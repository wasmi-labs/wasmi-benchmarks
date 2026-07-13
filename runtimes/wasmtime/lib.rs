#![crate_type = "dylib"]
#![cfg(any(feature = "cranelift", feature = "winch", feature = "pulley"))]

use benchmark_utils as utils;
use benchmark_utils::{
    CompileTestId, ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, TestId,
};
use wasmtime::{Func, Val, ValType};

#[derive(Debug, Copy, Clone)]
pub enum Strategy {
    Cranelift,
    Winch,
    Pulley,
}

pub struct Wasmtime {
    pub strategy: Strategy,
}

/// A concrete Wasmtime runtime with its linker, produced by [`Wasmtime::setup`].
struct WasmtimeInstance {
    linker: wasmtime::Linker<()>,
}

/// An instantiated Wasmtime module, produced by [`WasmtimeInstance::load`].
struct WasmtimeModule {
    store: wasmtime::Store<()>,
    instance: wasmtime::Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for Wasmtime {
    fn id(&self) -> &'static str {
        match self.strategy {
            Strategy::Cranelift => "wasmtime.cranelift",
            Strategy::Winch => "wasmtime.winch",
            Strategy::Pulley => "wasmtime.pulley",
        }
    }

    fn compile(&self, id: CompileTestId, _wasm: &[u8]) -> bool {
        if !self.can_run(id.into()) {
            return false;
        }
        #[cfg(any(feature = "cranelift", feature = "winch"))]
        {
            let engine = make_engine(self.strategy);
            wasmtime::Module::new(&engine, wasm).unwrap();
        }
        true
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let linker = wasmtime::Linker::new(&make_engine(self.strategy));
        Some(Box::new(WasmtimeInstance { linker }))
    }
}

impl Wasmtime {
    fn can_run(&self, id: TestId) -> bool {
        match self.strategy {
            Strategy::Cranelift => match id {
                // Note: ffmpeg takes too long to compile for Cranelift
                TestId::Compile(CompileTestId::Ffmpeg) => false,
                _ => true,
            },
            Strategy::Winch => match id {
                // Note: winch does not support the Wasm `tail-call` proposal.
                TestId::Execute(ExecuteTestId::FibonacciTail) => false,
                _ => {
                    // Note: winch only works on `x86_64` and `aarch64`.
                    cfg!(target_arch = "x86_64") || cfg!(target_arch = "aarch64")
                }
            },
            Strategy::Pulley => match id {
                // Note: ffmpeg takes too long to compile for Pulley
                TestId::Compile(CompileTestId::Ffmpeg) => false,
                _ => true,
            },
        }
    }
}

impl RuntimeInstance for WasmtimeInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        let ty = wasmtime::FuncType::new(
            self.linker.engine(),
            ty.params().iter().copied().map(to_wasmtime_valtype),
            ty.results().iter().copied().map(to_wasmtime_valtype),
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

    fn instantiate(self: Box<Self>, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let WasmtimeInstance { linker } = *self;
        let engine = linker.engine().clone();
        let mut store = <wasmtime::Store<()>>::new(&engine, ());
        let module = wasmtime::Module::new(&engine, wasm).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        Box::new(WasmtimeModule {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

fn make_engine(strategy: Strategy) -> wasmtime::Engine {
    let mut config = wasmtime::Config::default();
    if matches!(strategy, Strategy::Cranelift) {
        config.wasm_tail_call(true);
    }
    config.strategy(match strategy {
        Strategy::Cranelift | Strategy::Pulley => wasmtime::Strategy::Cranelift,
        Strategy::Winch => wasmtime::Strategy::Winch,
    });
    if matches!(strategy, Strategy::Pulley) {
        config.target("pulley64").unwrap();
    }
    wasmtime::Engine::new(&config).unwrap()
}

impl ModuleInstance for WasmtimeModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(&mut self.store, name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.ty(&self.store).params().len());
        assert_eq!(results.len(), func.ty(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results)?;
        Ok(())
    }
}

impl WasmtimeModule {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) {
        self.results.clear();
        for ty in func.ty(&self.store).results() {
            self.results.push(default_val(ty))
        }
    }

    fn write_back_results(&mut self, results: &mut [utils::Val]) -> anyhow::Result<()> {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            let ty = self.results[i].ty(&self.store)?;
            let src = core::mem::replace(&mut self.results[i], default_val(ty));
            *result = into_utils_val(src);
        }
        Ok(())
    }
}

fn to_wasmtime_valtype(ty: utils::ValType) -> ValType {
    match ty {
        utils::ValType::I32 => ValType::I32,
        utils::ValType::I64 => ValType::I64,
        utils::ValType::F32 => ValType::F32,
        utils::ValType::F64 => ValType::F64,
    }
}

fn default_val(ty: ValType) -> Val {
    Val::default_for_ty(&ty).unwrap()
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(val) => Val::I32(val),
        utils::Val::I64(val) => Val::I64(val),
        utils::Val::F32(val) => Val::F32(val.to_bits()),
        utils::Val::F64(val) => Val::F64(val.to_bits()),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(val) => utils::Val::I32(val),
        Val::I64(val) => utils::Val::I64(val),
        Val::F32(val) => utils::Val::F32(f32::from_bits(val)),
        Val::F64(val) => utils::Val::F64(f64::from_bits(val)),
        _ => panic!(),
    }
}
