#![crate_type = "dylib"]

use benchmark_utils::{self as utils, CompileTestId};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use wasmi::Func;
use wasmi::Val;

pub struct WasmiV032;

/// A concrete Wasmi runtime with its linker, produced by [`WasmiV032::setup`].
struct WasmiInstance {
    linker: wasmi::Linker<()>,
}

/// An instantiated Wasmi module, produced by [`WasmiInstance::load`].
struct WasmiModule {
    store: wasmi::Store<()>,
    instance: wasmi::Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for WasmiV032 {
    fn id(&self) -> &'static str {
        "wasmi-v0.32"
    }

    fn compile(&self, _id: CompileTestId, wasm: &[u8]) -> bool {
        let engine = make_engine();
        wasmi::Module::new(&engine, wasm).unwrap();
        true
    }

    fn setup(&self, _id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        let linker = wasmi::Linker::new(&make_engine());
        Some(Box::new(WasmiInstance { linker }))
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

    fn instantiate(self: Box<Self>, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let WasmiInstance { linker } = *self;
        let engine = linker.engine().clone();
        let mut store = <wasmi::Store<()>>::new(&engine, ());
        let module = wasmi::Module::new(&engine, wasm).unwrap();
        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();
        Box::new(WasmiModule {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

fn make_engine() -> wasmi::Engine {
    let mut config = wasmi::Config::default();
    config.wasm_tail_call(true);
    wasmi::Engine::new(&config)
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
            self.results.push(Val::default(*ty))
        }
    }

    fn write_back_results(&mut self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            let ty = self.results[i].ty();
            let src = core::mem::replace(&mut self.results[i], Val::default(ty));
            *result = into_utils_val(src);
        }
    }
}

fn to_wasmi_valtype(ty: utils::ValType) -> wasmi::core::ValType {
    match ty {
        utils::ValType::I32 => wasmi::core::ValType::I32,
        utils::ValType::I64 => wasmi::core::ValType::I64,
        utils::ValType::F32 => wasmi::core::ValType::F32,
        utils::ValType::F64 => wasmi::core::ValType::F64,
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
