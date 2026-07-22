#![crate_type = "dylib"]

use anyhow::{anyhow, bail};
use benchmark_utils::{
    self as utils, ModuleInstance, Runtime, RuntimeInstance, TestId,
};
use sf_nano_core::{BackendMode, Caller, Import, Instance, Value, WasmError, set_backend_mode};

/// The Silverfir-nano Wasm runtime.
///
/// Silverfir-nano is JIT-only: its single execution backend is native code generation
/// ([`BackendMode::Native`]), so there is just one configuration to benchmark.
pub struct SilverfirNano;

/// A Silverfir-nano runtime with its recorded host functions, produced by [`SilverfirNano::setup`].
///
/// Silverfir-nano's [`Instance::new`] takes all imports up front, so — like the `tinywasm` and
/// `stitch` adapters — host functions are recorded into a runtime-neutral [`Linker`](utils::Linker)
/// and replayed as [`Import`]s on every instantiation.
struct SilverfirNanoInstance {
    linker: utils::Linker,
}

/// An instantiated Silverfir-nano module, produced by [`SilverfirNanoInstance::instantiate`].
struct SilverfirNanoModule {
    instance: Instance,
    params: Vec<Value>,
}

impl Runtime for SilverfirNano {
    fn id(&self) -> &'static str {
        "silverfir-nano"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        // Idempotent global; `Native` is already the default. Set it explicitly so the choice of
        // backend is visible at the adapter boundary.
        set_backend_mode(BackendMode::Native);
        Some(Box::new(SilverfirNanoInstance {
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
        // Recorded here and replayed as a real Silverfir-nano import in `instantiate`, where each
        // call is dispatched to `func`. In practice the benchmarks never call these (execute cases
        // import nothing; startup cases only link imports to satisfy instantiation, which is all
        // that is timed), but the wiring is faithful rather than an inert stub.
        self.linker.define(module, name, ty, func);
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        // Replay every recorded host function as a real import. Silverfir-nano now accepts `Fn`
        // closures for host functions, so each import captures the recorded `func` and dispatches
        // to it across the runtime-neutral value boundary instead of being a no-op stub.
        let imports: Vec<Import> = self
            .linker
            .funcs()
            .map(|(module, name, ty, func)| {
                // Owned so the `'static` host closure can seed its result slots on every call.
                let result_types = ty.results().to_vec();
                Import::func(
                    module,
                    name,
                    move |_caller: &mut Caller, params: &[Value], results: &mut [Value]| {
                        dispatch_host_func(func, &result_types, params, results)
                    },
                )
            })
            .collect();
        let instance = Instance::new(wasm, &imports).expect("failed to instantiate Wasm module");
        Box::new(SilverfirNanoModule {
            instance,
            params: Vec::new(),
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
        let call_results = self
            .instance
            .invoke(name, &self.params)
            .map_err(|err| anyhow!("silverfir-nano: call to `{name}` failed: {err}"))?;
        if call_results.len() != results.len() {
            bail!(
                "silverfir-nano: `{name}` returned {} results but {} were expected",
                call_results.len(),
                results.len(),
            );
        }
        for (dst, src) in results.iter_mut().zip(call_results) {
            *dst = into_utils_val(src)?;
        }
        Ok(())
    }

    fn read_memory(&mut self, _name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let memory = self
            .instance
            .memory()
            .ok_or_else(|| anyhow!("silverfir-nano: module has no memory"))?;
        let slice = mem_slice(memory, ptr, buffer.len())?;
        buffer.copy_from_slice(slice);
        Ok(())
    }

    fn write_memory(&mut self, _name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let memory = self
            .instance
            .memory_mut()
            .ok_or_else(|| anyhow!("silverfir-nano: module has no memory"))?;
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
            return Err(WasmError::Trap(
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
