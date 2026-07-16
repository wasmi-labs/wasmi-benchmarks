#![crate_type = "dylib"]

use anyhow::bail;
use benchmark_utils::{self as utils};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use std::collections::BTreeMap;
use wasmedge::{
    AsInstance, CallingFrame, Config, Executor, FuncType, Function, ImportModule, Instance, Loader,
    Store, Validator, WasmValue as Val,
};
use wasmedge_types::ValType;
use wasmedge_types::error::CoreError;

pub struct WasmEdge;

struct WasmEdgeInstance {
    imports: BTreeMap<Box<str>, ImportModule<()>>,
    datas: Vec<*mut HostFuncWrapperData>,
}

impl Drop for WasmEdgeInstance {
    fn drop(&mut self) {
        for &mut data in &mut self.datas {
            _ = unsafe { Box::from_raw(data) };
        }
    }
}

struct WasmEdgeModule {
    executor: Executor,
    instance: Instance,
    /// Kept alive: `executor`/`instance` reference state owned by the store.
    #[allow(dead_code)]
    store: Store,
    params: Vec<Val>,
}

impl Runtime for WasmEdge {
    fn id(&self) -> &'static str {
        "wasmedge"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        Some(Box::new(WasmEdgeInstance {
            datas: Vec::new(),
            imports: BTreeMap::new(),
        }))
    }
}

impl WasmEdge {
    fn can_run(&self, _id: TestId) -> bool {
        true
    }
}

struct HostFuncWrapperData {
    func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    func_ty: utils::FuncType,
}

impl HostFuncWrapperData {
    fn new(
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
        ty: utils::FuncType,
    ) -> Self {
        Self { func, func_ty: ty }
    }
}

fn host_func_wrapper(
    data: &mut HostFuncWrapperData,
    _instance: &mut Instance,
    _frame: &mut CallingFrame,
    params: Vec<Val>,
) -> Result<Vec<Val>, CoreError> {
    let params: Vec<_> = params.iter().copied().map(into_utils_val).collect();
    let mut results: Vec<_> = data
        .func_ty
        .results()
        .iter()
        .copied()
        .map(utils::Val::default_for_ty)
        .collect();
    (data.func)(&params[..], &mut results[..]);
    Ok(results.into_iter().map(from_utils_val).collect())
}

impl RuntimeInstance for WasmEdgeInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        self.imports
            .entry(module.into())
            .or_insert(ImportModule::create(module, Box::new(())).unwrap())
            .add_func(name, {
                let func_ty = from_utils_func_type(&ty);
                let data = Box::leak(Box::new(HostFuncWrapperData::new(func, ty)));
                self.datas.push(data as _);
                unsafe {
                    Function::create_sync_func(&func_ty, host_func_wrapper, data as _, 0).unwrap()
                }
            });
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let mut config = Config::create().unwrap();
        config.tail_call(true);
        let mut executor = Executor::create(Some(&config), None).unwrap();
        let mut store = Store::create().unwrap();
        for import_module in self.imports.values() {
            executor
                .register_import_module(&mut store, import_module)
                .unwrap();
        }
        let loader = Loader::create(Some(&config)).unwrap();
        let module = loader.from_bytes(wasm).unwrap();
        let validator = Validator::create(Some(&config)).unwrap();
        validator.validate(&module).unwrap();
        let instance = executor
            .register_named_module(&mut store, &module, "benchmark_module")
            .unwrap();
        Box::new(WasmEdgeModule {
            executor,
            instance,
            store,
            params: Vec::new(),
        })
    }
}

impl ModuleInstance for WasmEdgeModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let mut func = self.instance.get_func_mut(name).unwrap();
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
        let call_results = self
            .executor
            .call_func(&mut func, self.params.iter().copied())
            .unwrap();
        for (dst, src) in results.iter_mut().zip(call_results) {
            *dst = into_utils_val(src);
        }
        Ok(())
    }

    fn read_memory(&self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let memory = self.instance.get_memory_ref(name)?;
        let Some(bytes) = memory.slice::<u8>(ptr as usize, buffer.len()) else {
            bail!(
                "failed to slice memory at {ptr} with length {}",
                buffer.len()
            )
        };
        buffer.copy_from_slice(bytes);
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let memory = self.instance.get_memory_mut(name)?;
        let Some(bytes) = memory.mut_slice::<u8>(ptr as usize, buffer.len()) else {
            bail!(
                "failed to slice memory at {ptr} with length {}",
                buffer.len()
            )
        };
        bytes.copy_from_slice(buffer);
        Ok(())
    }
}

fn to_wasmedge_valtype(ty: utils::ValType) -> ValType {
    match ty {
        utils::ValType::I32 => ValType::I32,
        utils::ValType::I64 => ValType::I64,
        utils::ValType::F32 => ValType::F32,
        utils::ValType::F64 => ValType::F64,
    }
}

fn from_utils_func_type(ty: &utils::FuncType) -> FuncType {
    FuncType::new(
        ty.params()
            .iter()
            .copied()
            .map(to_wasmedge_valtype)
            .collect(),
        ty.results()
            .iter()
            .copied()
            .map(to_wasmedge_valtype)
            .collect(),
    )
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(val) => Val::from_i32(val),
        utils::Val::I64(val) => Val::from_i64(val),
        utils::Val::F32(val) => Val::from_f32(val),
        utils::Val::F64(val) => Val::from_f64(val),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val.ty() {
        ValType::I32 => utils::Val::I32(val.to_i32()),
        ValType::I64 => utils::Val::I64(val.to_i64()),
        ValType::F32 => utils::Val::F32(val.to_f32()),
        ValType::F64 => utils::Val::F64(val.to_f64()),
        _ => unreachable!(),
    }
}
