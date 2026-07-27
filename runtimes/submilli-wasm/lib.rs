#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils as utils;
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use submilli_wasm::{Val, ValType};

pub struct SubmilliWasm;

/// A concrete Submilli-wasm runtime with its linker, produced by [`SubmilliWasm::setup`].
struct SubmilliWasmInstance {
    linker: submilli_wasm::Linker<()>,
}

/// An instantiated Submilli-wasm module, produced by [`SubmilliWasmInstance::instantiate`].
struct SubmilliWasmModule {
    store: submilli_wasm::Store<()>,
    instance: submilli_wasm::Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for SubmilliWasm {
    fn id(&self) -> &'static str {
        "submilli-wasm"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let linker = submilli_wasm::Linker::new(&make_engine());
        Some(Box::new(SubmilliWasmInstance { linker }))
    }
}

impl SubmilliWasm {
    fn can_run(&self, _id: TestId) -> bool {
        true
    }
}

impl RuntimeInstance for SubmilliWasmInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        let result_tys: Vec<utils::ValType> = ty.results().to_vec();
        let ty = submilli_wasm::FuncType::new(
            self.linker.engine(),
            ty.params().iter().copied().map(to_submilli_valtype),
            ty.results().iter().copied().map(to_submilli_valtype),
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
        let mut store = <submilli_wasm::Store<()>>::new(&engine, ());
        let module = submilli_wasm::Module::new(&engine, wasm).unwrap();
        let instance = self.linker.instantiate(&mut store, &module).unwrap();
        Box::new(SubmilliWasmModule {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

fn make_engine() -> submilli_wasm::Engine {
    let mut config = submilli_wasm::Config::default();
    config.wasm_tail_call(true);
    submilli_wasm::Engine::new(&config).unwrap()
}

impl ModuleInstance for SubmilliWasmModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.get_func(&mut self.store, name) else {
            anyhow::bail!("failed to find function")
        };
        let ty = func.ty(&self.store);
        assert_eq!(params.len(), ty.params().len());
        assert_eq!(results.len(), ty.results().len());
        self.prepare_params(params);
        self.prepare_results(ty);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(&mut self.store, name) else {
            bail!("memory not found: {name}")
        };
        memory.read(&self.store, ptr as usize, buffer)?;
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let Some(memory) = self.instance.get_memory(&mut self.store, name) else {
            bail!("memory not found: {name}")
        };
        memory.write(&mut self.store, ptr as usize, buffer)?;
        Ok(())
    }
}

impl SubmilliWasmModule {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, ty: submilli_wasm::FuncType) {
        self.results.clear();
        self.results.extend(ty.results().map(default_val));
    }

    fn write_back_results(&mut self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (result, src) in results.iter_mut().zip(self.results.iter().copied()) {
            *result = into_utils_val(src);
        }
    }
}

fn to_submilli_valtype(ty: utils::ValType) -> ValType {
    match ty {
        utils::ValType::I32 => ValType::I32,
        utils::ValType::I64 => ValType::I64,
        utils::ValType::F32 => ValType::F32,
        utils::ValType::F64 => ValType::F64,
    }
}

fn default_val(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0),
        ValType::F64 => Val::F64(0),
        unsupported => panic!("unsupported value type: {unsupported:?}"),
    }
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
