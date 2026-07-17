#![crate_type = "dylib"]

use anyhow::{anyhow, bail};
use benchmark_utils::{self as utils};
use benchmark_utils::{ExecuteTestId, HostFunc, ModuleInstance, Runtime, RuntimeInstance, TestId};
use checked::{Linker, Store, Stored, StoredExternVal, StoredRunState, StoredValue};
use wasm::{F32, F64, FuncType, ModuleAddr, NumType, ResultType, ValType, decode_and_validate};

pub struct DlrWasmInterpreter;

struct DlrInstance {
    /// Note: the interpreter allocates host functions inside a [`Store`], so rather than holding a live store
    ///       the recorded host functions are replayed into a fresh store on every instantiation.
    linker: utils::Linker,
}

struct DlrModule {
    /// The interpreter store. Its `'static` lifetime is fabricated: it actually borrows into
    /// `bytes`. Declared **first** so it is dropped before `bytes` (fields drop in declaration
    /// order), guaranteeing the borrowed bytecode outlives the store.
    store: Store<'static, ()>,
    /// The instantiated module inside `store`.
    module_addr: Stored<ModuleAddr>,
    /// The recorded host functions, indexed by the `hostcode` assigned in [`DlrInstance::instantiate`].
    ///
    /// The DLR interpreter uses a *returning* host-function model: when Wasm calls an import,
    /// execution returns to us with the `hostcode` (an index into this vector) and the arguments.
    stubs: Vec<(HostFunc, utils::FuncType)>,
    /// Owns the module bytecode that `store` borrows from. Must outlive `store` (see above).
    #[allow(
        dead_code,
        reason = "kept alive so `store`'s in-place references stay valid"
    )]
    bytes: Box<[u8]>,
}

impl Runtime for DlrWasmInterpreter {
    fn id(&self) -> &'static str {
        "dlr-wasm-interpreter"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !Self::can_run(id) {
            return None;
        }
        Some(Box::new(DlrInstance {
            linker: utils::Linker::new(),
        }))
    }
}

impl DlrWasmInterpreter {
    fn can_run(id: TestId) -> bool {
        !matches!(id, TestId::Execute(ExecuteTestId::FibonacciTail))
    }
}

impl RuntimeInstance for DlrInstance {
    fn link_func(&mut self, module: &str, name: &str, ty: utils::FuncType, func: HostFunc) {
        self.linker.define(module, name, ty, func);
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        // Note: the DLR interpreter runs in-place, so its `Store` keeps references into the bytecode for
        //       its whole life. We own the bytecode in a `Box<[u8]>` and hand the interpreter a `'static`
        //       view of it; `DlrModule` keeps the box alive and drops the store before it (field order).
        let bytes: Box<[u8]> = Box::from(wasm);
        // SAFETY: `bytes` is moved into the returned `DlrModule` and is never mutated or moved out
        //         of while `store` (which borrows it) is alive. `store` is declared before `bytes` in
        //         `DlrModule`, so it is dropped first. Moving the box into the struct keeps the heap data at
        //         a stable address, so the references handed to the interpreter stay valid.
        let bytes_static: &'static [u8] =
            unsafe { core::slice::from_raw_parts(bytes.as_ptr(), bytes.len()) };

        let module = decode_and_validate(bytes_static, &mut ())
            .unwrap_or_else(|err| panic!("dlr-wasm-interpreter: validation failed: {err:?}"));

        let mut store = Store::new(());
        let mut linker = Linker::new();

        // Note: allocate one host function per recorded import, using its index as the `hostcode`, and
        //       bind it into the linker by name. The index lets us dispatch back to the right stub when
        //       the interpreter returns control on a host call.
        let mut stubs: Vec<(HostFunc, utils::FuncType)> = Vec::new();
        for (i, (module_name, name, ty, func)) in self.linker.funcs().enumerate() {
            let func_addr = store.func_alloc(from_utils_func_type(ty), i);
            linker
                .define(
                    module_name.to_string(),
                    name.to_string(),
                    StoredExternVal::Func(func_addr),
                )
                .unwrap_or_else(|err| {
                    panic!(
                        "dlr-wasm-interpreter: failed to define `{module_name}::{name}`: {err:?}"
                    )
                });
            stubs.push((func, ty.clone()));
        }

        let module_addr = linker
            .module_instantiate(&mut store, &module, None)
            .expect("dlr-wasm-interpreter: module imports could not be resolved")
            .unwrap_or_else(|err| panic!("dlr-wasm-interpreter: instantiation failed: {err:?}"))
            .module_addr;

        Box::new(DlrModule {
            store,
            module_addr,
            stubs,
            bytes,
        })
    }
}

impl ModuleInstance for DlrModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let func = self
            .store
            .instance_export(self.module_addr, name)
            .map_err(|err| anyhow!("dlr-wasm-interpreter: no export `{name}`: {err:?}"))?
            .as_func()
            .ok_or_else(|| anyhow!("dlr-wasm-interpreter: export `{name}` is not a function"))?;

        let call_params: Vec<StoredValue> = params.iter().copied().map(from_utils_val).collect();
        let mut state = self
            .store
            .invoke(func, call_params, None)
            .map_err(|err| anyhow!("dlr-wasm-interpreter: calling `{name}` failed: {err:?}"))?;

        // Drive the interpreter to completion. It returns control to us on every host call (the
        // "returning" host-function model), which we service from the recorded stubs and resume.
        loop {
            match state {
                StoredRunState::Finished { values, .. } => {
                    if values.len() != results.len() {
                        bail!(
                            "dlr-wasm-interpreter: `{name}` returned {} results, expected {}",
                            values.len(),
                            results.len(),
                        );
                    }
                    for (dst, val) in results.iter_mut().zip(values) {
                        *dst = into_utils_val(val)?;
                    }
                    return Ok(());
                }
                StoredRunState::HostCalled {
                    host_call,
                    resumable,
                } => {
                    let (func, ty) = &self.stubs[host_call.hostcode];
                    let func = *func;
                    let mut host_results: Vec<utils::Val> = ty
                        .results()
                        .iter()
                        .copied()
                        .map(utils::Val::default_for_ty)
                        .collect();
                    let host_params = host_call
                        .params
                        .iter()
                        .copied()
                        .map(into_utils_val)
                        .collect::<anyhow::Result<Vec<_>>>()?;
                    func(&host_params, &mut host_results);
                    let ret: Vec<StoredValue> =
                        host_results.into_iter().map(from_utils_val).collect();
                    state = self.store.finish_host_call(resumable, ret).map_err(|err| {
                        anyhow!("dlr-wasm-interpreter: host call failed: {err:?}")
                    })?;
                }
                StoredRunState::Resumable {
                    resumable,
                    required_fuel,
                } => {
                    // Note: `Resumable` is overloaded: `finish_host_call` returns it (with
                    //       `required_fuel == None`) to hand control back so execution continues after a
                    //       serviced host call. A `Some` value means genuine fuel exhaustion, which cannot
                    //       happen here because we always invoke unfueled (`None`).
                    if required_fuel.is_some() {
                        bail!("dlr-wasm-interpreter: `{name}` ran out of fuel unexpectedly");
                    }
                    state = self.store.resume_wasm(resumable).map_err(|err| {
                        anyhow!("dlr-wasm-interpreter: resuming `{name}` failed: {err:?}")
                    })?;
                }
            }
        }
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let memory = self
            .store
            .instance_export(self.module_addr, name)
            .map_err(|err| anyhow!("dlr-wasm-interpreter: no memory export `{name}`: {err:?}"))?
            .as_mem()
            .ok_or_else(|| anyhow!("dlr-wasm-interpreter: export `{name}` is not a memory"))?;
        // Note: `read_memory` is `&self`, and the DLR API exposes only a per-byte immutable read
        //       (`mem_read`) — there is no immutable bulk accessor. This path is only exercised by the
        //       `compression` execute test, outside the timed loop.
        for (offset, dst) in buffer.iter_mut().enumerate() {
            let addr = ptr + offset as u32;
            *dst = self.store.mem_read(memory, addr).map_err(|err| {
                anyhow!("dlr-wasm-interpreter: reading `{name}` at {addr}: {err:?}")
            })?;
        }
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let memory = self
            .store
            .instance_export(self.module_addr, name)
            .map_err(|err| anyhow!("dlr-wasm-interpreter: no memory export `{name}`: {err:?}"))?
            .as_mem()
            .ok_or_else(|| anyhow!("dlr-wasm-interpreter: export `{name}` is not a memory"))?;
        let data = self.store.mem_data_mut(memory);
        let ptr = ptr as usize;
        let len = buffer.len();
        let Some(dst) = data.get_mut(ptr..ptr + len) else {
            bail!("dlr-wasm-interpreter: cannot write {len} bytes to `{name}` at {ptr}");
        };
        dst.copy_from_slice(buffer);
        Ok(())
    }
}

fn from_utils_func_type(ty: &utils::FuncType) -> FuncType {
    FuncType {
        params: ResultType {
            valtypes: ty
                .params()
                .iter()
                .copied()
                .map(from_utils_val_type)
                .collect(),
        },
        returns: ResultType {
            valtypes: ty
                .results()
                .iter()
                .copied()
                .map(from_utils_val_type)
                .collect(),
        },
    }
}

/// Converts a runtime-neutral [`utils::ValType`] into the interpreter's [`ValType`].
fn from_utils_val_type(ty: utils::ValType) -> ValType {
    ValType::NumType(match ty {
        utils::ValType::I32 => NumType::I32,
        utils::ValType::I64 => NumType::I64,
        utils::ValType::F32 => NumType::F32,
        utils::ValType::F64 => NumType::F64,
    })
}

/// Converts a runtime-neutral [`utils::Val`] into the interpreter's [`StoredValue`].
///
/// The `as` casts are bit-preserving reinterpretations (`i32`/`i64` bits stored as `u32`/`u64`).
fn from_utils_val(val: utils::Val) -> StoredValue {
    match val {
        utils::Val::I32(val) => StoredValue::I32(val as u32),
        utils::Val::I64(val) => StoredValue::I64(val as u64),
        utils::Val::F32(val) => StoredValue::F32(F32(val)),
        utils::Val::F64(val) => StoredValue::F64(F64(val)),
    }
}

/// Converts the interpreter's [`StoredValue`] into a runtime-neutral [`utils::Val`].
///
/// The benchmark suite only uses the four MVP numeric types, so `V128`/`Ref` values are rejected.
fn into_utils_val(val: StoredValue) -> anyhow::Result<utils::Val> {
    Ok(match val {
        StoredValue::I32(bits) => utils::Val::I32(bits as i32),
        StoredValue::I64(bits) => utils::Val::I64(bits as i64),
        StoredValue::F32(F32(val)) => utils::Val::F32(val),
        StoredValue::F64(F64(val)) => utils::Val::F64(val),
        other => bail!("dlr-wasm-interpreter: unsupported value type: {other:?}"),
    })
}
