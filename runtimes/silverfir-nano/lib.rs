#![crate_type = "dylib"]
#![cfg(any(feature = "jit", feature = "interp"))]

use anyhow::{anyhow, bail};
use benchmark_utils::{self as utils, ModuleInstance, Runtime, RuntimeInstance, TestId};
pub use sf_nano_core::Tier;
use sf_nano_core::value_type::ValueType;
use sf_nano_core::{Caller, Config, Engine, FunctionType, Import, Instance, Value, WasmError};

/// The Silverfir-nano Wasm runtime.
///
/// Silverfir-nano ships two execution engines — an optimizing JIT and an interpreter — selected
/// per [`Engine`] via its [`Tier`], so both are benchmarked as separate configurations.
pub struct SilverfirNano {
    pub tier: Tier,
}

/// A Silverfir-nano runtime with its engine and recorded host functions, produced by
/// [`SilverfirNano::setup`].
///
/// Silverfir-nano's [`Instance::new`] takes all imports up front, so — like the `tinywasm` and
/// `stitch` adapters — host functions are recorded into a runtime-neutral [`Linker`](utils::Linker)
/// and replayed as [`Import`]s on every instantiation.
struct SilverfirNanoInstance {
    engine: Engine,
    linker: utils::Linker,
}

/// An instantiated Silverfir-nano module, produced by [`SilverfirNanoInstance::instantiate`].
struct SilverfirNanoModule {
    instance: Instance,
    params: Vec<Value>,
    results: Vec<Value>,
}

impl Runtime for SilverfirNano {
    fn id(&self) -> &'static str {
        match self.tier {
            #[cfg(feature = "jit")]
            Tier::Jit => "silverfir-nano.jit",
            #[cfg(feature = "interp")]
            Tier::Interp => "silverfir-nano.interpreter",
        }
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        // Compile single-threaded: by default Silverfir-nano spreads eager JIT compilation of
        // large modules over multiple threads, unlike every other runtime here. A no-op for the
        // interpreter, which has nothing to parallelize.
        let config = Config::new().tier(self.tier).parallel_compilation(false);
        let engine = Engine::new(config).expect("failed to configure Silverfir-nano engine");
        Some(Box::new(SilverfirNanoInstance {
            engine,
            linker: utils::Linker::new(),
        }))
    }
}

impl SilverfirNano {
    fn can_run(&self, _id: TestId) -> bool {
        true
    }
}

impl RuntimeInstance for SilverfirNanoInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        // Capture each host function and its signature for instantiation.
        self.linker.define(module, name, ty, func);
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let imports: Vec<Import> = self
            .linker
            .funcs()
            .map(|(module, name, ty, func)| {
                // Owned so the `'static` host closure can seed its result slots on every call.
                let result_types = ty.results().to_vec();
                let func_type = FunctionType::new(
                    ty.params().iter().copied().map(from_utils_type).collect(),
                    ty.results().iter().copied().map(from_utils_type).collect(),
                );
                Import::func_typed(
                    module,
                    name,
                    move |_caller: &mut Caller, params: &[Value], results: &mut [Value]| {
                        dispatch_host_func(func, &result_types, params, results)
                    },
                    func_type,
                )
            })
            .collect();
        let instance =
            Instance::new(&self.engine, wasm, &imports).expect("failed to instantiate Wasm module");
        Box::new(SilverfirNanoModule {
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

impl ModuleInstance for SilverfirNanoModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
        let func = self
            .instance
            .get_func(name)
            .ok_or_else(|| anyhow!("silverfir-nano: function `{name}` not found"))?;
        self.results.resize(results.len(), Value::I32(0));
        self.instance
            .call(&func, &self.params, &mut self.results)
            .map_err(|err| anyhow!("silverfir-nano: call to `{name}` failed: {err}"))?;
        for (dst, src) in results.iter_mut().zip(self.results.iter().copied()) {
            *dst = into_utils_val(src)?;
        }
        Ok(())
    }

    fn read_memory(&mut self, _name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let memory = self
            .instance
            .memory()
            .map_err(|err| anyhow!("silverfir-nano: memory access failed: {err}"))?;
        let slice = mem_slice(&memory, ptr, buffer.len())?;
        buffer.copy_from_slice(slice);
        Ok(())
    }

    fn write_memory(&mut self, _name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let mut memory = self
            .instance
            .memory_mut()
            .map_err(|err| anyhow!("silverfir-nano: memory access failed: {err}"))?;
        let len = memory.len();
        let start = ptr as usize;
        let end = start
            .checked_add(buffer.len())
            .filter(|&end| end <= len)
            .ok_or_else(|| anyhow!("silverfir-nano: memory write out of bounds"))?;
        memory[start..end].copy_from_slice(buffer);
        Ok(())
    }
}

/// Dispatches a call to a recorded host `func` across the runtime-neutral value boundary.
///
/// Silverfir-nano hands the host closure a `results` slice pre-sized to the callee's result arity
/// (each slot defaulted). The recorded `func` writes into a matching runtime-neutral buffer, seeded
/// from `result_types`, which is then converted back into `results`.
fn dispatch_host_func(
    func: utils::HostFunc,
    result_types: &[utils::ValType],
    params: &[Value],
    results: &mut [Value],
) -> Result<(), WasmError> {
    let params = params
        .iter()
        .copied()
        .map(host_value_to_utils)
        .collect::<Result<Vec<_>, _>>()?;
    let mut out: Vec<utils::Val> = result_types
        .iter()
        .copied()
        .map(utils::Val::default_for_ty)
        .collect();
    func(&params, &mut out);
    for (dst, src) in results.iter_mut().zip(out) {
        *dst = from_utils_val(src);
    }
    Ok(())
}

/// Converts a Silverfir-nano [`Value`] host-function argument into the runtime-neutral
/// [`Val`](utils::Val). Traps on `V128`/reference arguments, which the numeric benchmark imports
/// never use.
fn host_value_to_utils(val: Value) -> Result<utils::Val, WasmError> {
    Ok(match val {
        Value::I32(val) => utils::Val::I32(val),
        Value::I64(val) => utils::Val::I64(val),
        Value::F32(val) => utils::Val::F32(val),
        Value::F64(val) => utils::Val::F64(val),
        _ => {
            return Err(WasmError::trap(
                "silverfir-nano: unsupported host function argument type",
            ));
        }
    })
}

/// Returns `memory[ptr..ptr + len]`, erroring if the range is out of bounds.
fn mem_slice(memory: &[u8], ptr: u32, len: usize) -> anyhow::Result<&[u8]> {
    let start = ptr as usize;
    let end = start
        .checked_add(len)
        .filter(|&end| end <= memory.len())
        .ok_or_else(|| anyhow!("silverfir-nano: memory read out of bounds"))?;
    Ok(&memory[start..end])
}

fn from_utils_val(val: utils::Val) -> Value {
    match val {
        utils::Val::I32(val) => Value::I32(val),
        utils::Val::I64(val) => Value::I64(val),
        utils::Val::F32(val) => Value::F32(val),
        utils::Val::F64(val) => Value::F64(val),
    }
}

fn into_utils_val(val: Value) -> anyhow::Result<utils::Val> {
    Ok(match val {
        Value::I32(val) => utils::Val::I32(val),
        Value::I64(val) => utils::Val::I64(val),
        Value::F32(val) => utils::Val::F32(val),
        Value::F64(val) => utils::Val::F64(val),
        other => bail!("silverfir-nano: unsupported result value: {other:?}"),
    })
}

fn from_utils_type(ty: utils::ValType) -> ValueType {
    match ty {
        utils::ValType::I32 => ValueType::I32,
        utils::ValType::I64 => ValueType::I64,
        utils::ValType::F32 => ValueType::F32,
        utils::ValType::F64 => ValueType::F64,
    }
}
