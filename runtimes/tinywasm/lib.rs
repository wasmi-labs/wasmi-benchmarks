#![crate_type = "dylib"]

use benchmark_utils as utils;
use benchmark_utils::{BenchInstance, BenchRuntime, ExecuteTestId, TestId, elapsed_ms};
use tinywasm::types::WasmValue as Val;

pub struct Tinywasm;

struct TinywasmRuntime {
    store: tinywasm::Store,
    instance: tinywasm::ModuleInstance,
    params: Vec<Val>,
}

impl BenchRuntime for Tinywasm {
    fn id(&self) -> &'static str {
        "tinywasm"
    }

    fn can_run(&self, id: TestId) -> bool {
        !matches!(id, TestId::Execute(ExecuteTestId::FibonacciTail))
    }

    fn compile(&self, wasm: &[u8]) {
        tinywasm::parse_bytes(wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let mut store = tinywasm::Store::default();
        let module = tinywasm::parse_bytes(wasm).unwrap();
        let instance = tinywasm::ModuleInstance::instantiate(&mut store, &module, None).unwrap();
        Box::new(TinywasmRuntime {
            store,
            instance,
            params: Vec::new(),
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let mut store = tinywasm::Store::default();
        let module = tinywasm::parse_bytes(wasm).unwrap();
        let mut imports = tinywasm::Imports::new();
        imports.define(
            "env",
            "clock_ms",
            tinywasm::HostFunction::from(&mut store, |_ctx, _arg: ()| Ok(elapsed_ms() as i32)),
        );
        let instance =
            tinywasm::ModuleInstance::instantiate(&mut store, &module, Some(imports)).unwrap();
        let func = instance.func::<(), f32>(&store, "run").unwrap();
        func.call(&mut store, ()).unwrap()
    }
}

impl BenchInstance for TinywasmRuntime {
    fn call(&mut self, input: i64) {
        self.instance
            .func::<i64, i64>(&self.store, "run")
            .unwrap()
            .call(&mut self.store, input)
            .unwrap();
    }

    fn call_with(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let func = self.instance.func_untyped(&self.store, name)?;
        self.prepare_params(params);
        let call_results = func.call(&mut self.store, &self.params[..])?;
        self.write_back_results(results, &call_results[..]);
        Ok(())
    }
}

impl TinywasmRuntime {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn write_back_results(&self, dst: &mut [utils::Val], src: &[Val]) {
        assert_eq!(dst.len(), src.len());
        for (i, result) in dst.iter_mut().enumerate() {
            *result = into_utils_val(src[i]);
        }
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
        _ => panic!(),
    }
}
