#![crate_type = "dylib"]

use anyhow::{anyhow, bail};
use benchmark_utils::{self as utils, ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, StartupTestId, TestId};
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
    fn can_run(&self, id: TestId) -> bool {
        // Silverfir-nano is a Wasm 3.0 runtime (tail calls included) and the benchmarks it can't
        // handle are pruned here as they surface. Nothing is excluded yet.
        !matches!(id, TestId::Execute(ExecuteTestId::CoreMark) | TestId::Startup(StartupTestId::Ffmpeg))
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
        // The recorded `func` is never invoked: execute benchmarks import nothing, and startup
        // benchmarks link imports purely to satisfy instantiation (which is all that is timed).
        self.linker.define(module, name, ty, func);
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        // Replay every recorded import as an inert host stub. The import's type is taken from the
        // module's own import declaration, so an untyped `Import::func` is sufficient.
        let imports: Vec<Import> = self
            .linker
            .funcs()
            .map(|(module, name, _ty, _func)| Import::func(module, name, inert_stub))
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

/// Inert host import stub. Never actually invoked by the benchmarks (see [`link_func`]); it exists
/// only so modules with imports can be instantiated.
fn inert_stub(
    _caller: &mut Caller,
    _params: &[Value],
    _results: &mut [Value],
) -> Result<(), WasmError> {
    Ok(())
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
