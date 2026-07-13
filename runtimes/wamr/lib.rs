#![crate_type = "dylib"]

//! Adapter running the [WAMR] (WebAssembly Micro Runtime) interpreter in its
//! **fast-interpreter** configuration (WAMR's default interpreter build).
//!
//! [WAMR]: https://github.com/bytecodealliance/wasm-micro-runtime
//!
//! ## Host functions
//!
//! WAMR host functions are C function pointers and cannot capture Rust state, so the generic
//! `fn(&[Val], &mut [Val])` handed to [`RuntimeInstance::link_func`] cannot be turned into a
//! closure the way the other adapters do. Instead every host function is registered as a WAMR
//! [*raw* native function][raw] ([`wasm_runtime_register_natives_raw`]) that shares a single
//! [`host_trampoline`], and the per-function state (its [`FuncType`](utils::FuncType) and the
//! recorded `fn` pointer) is passed through WAMR's native-symbol *attachment*. The trampoline
//! recovers that state via [`wasm_runtime_get_function_attachment`], decodes the raw argument
//! buffer into [`Val`]s, calls the real host function, and writes the result back — so arbitrary
//! host functions with arbitrary signatures are supported, not just the ones used today.
//!
//! [raw]: https://github.com/bytecodealliance/wasm-micro-runtime/blob/main/doc/export_native_api.md

use benchmark_utils::{self as utils, ModuleInstance, Runtime, RuntimeInstance, TestId};
use benchmark_utils::{ExecuteTestId, HostFunc, Val, ValType};
use std::ffi::{CString, c_void};
use std::ptr;
use wamr_rust_sdk::{
    function::Function, instance::Instance, module::Module, runtime::Runtime as WamrRuntime,
    value::WasmValue,
};
use wamr_sys::{
    NativeSymbol, wasm_exec_env_t, wasm_runtime_get_function_attachment,
    wasm_runtime_register_natives_raw,
};

/// The Wasm operand stack size (in bytes) used when instantiating a module.
///
/// Chosen generously so the deeper recursive/heavy benchmark cases (e.g. `fibonacci-rec`,
/// `matrix-multiply`, `argon2`) do not exhaust the interpreter stack.
const STACK_SIZE: u32 = 8 * 1024 * 1024;

/// The WAMR runtime in its fast-interpreter configuration.
pub struct Wamr;

impl Runtime for Wamr {
    fn id(&self) -> &'static str {
        "wamr"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        let runtime = WamrRuntime::new().expect("WAMR: failed to create runtime");
        Some(Box::new(WamrInstance {
            runtime,
            hosts: Vec::new(),
        }))
    }
}

impl Wamr {
    fn can_run(&self, id: TestId) -> bool {
        !matches!(
            id,
            // WAMR's default fast-interpreter build has no tail-call support, so the `return_call`
            // based module fails to load (Wasm3 excludes it for the same reason).
            TestId::Execute(ExecuteTestId::FibonacciTail)
        )
    }
}

/// A host function registered on a [`WamrInstance`].
///
/// Boxed and kept alive for the runtime's lifetime because its address is handed to WAMR as a
/// native-symbol *attachment* and dereferenced by [`host_trampoline`] on every call.
struct HostFuncEntry {
    ty: utils::FuncType,
    func: HostFunc,
}

/// A configured WAMR runtime together with its registered host functions, produced by
/// [`Wamr::setup`].
///
/// WAMR registers host functions globally on the runtime and keeps raw pointers into the symbol
/// names and (via the attachment) the [`HostFuncEntry`]s, so those are stored here to outlive the
/// runtime. Each import is registered on its own, which keeps every pointer stable (no `Vec`
/// reallocation moves the boxed payloads) and avoids WAMR's requirement that multi-symbol arrays be
/// sorted by name.
struct WamrInstance {
    // `runtime` is declared first so it is dropped (and `wasm_runtime_destroy`d) before the host
    // function payloads it still references are freed.
    runtime: WamrRuntime,
    hosts: Vec<RegisteredHost>,
}

/// The kept-alive backing storage of one registered host function.
struct RegisteredHost {
    // `module`/`symbol` back the `*const c_char` names stored in `NativeSymbol`; `entry` backs the
    // attachment pointer; `symbols` is the single-element array WAMR keeps a pointer to. Boxing
    // gives each a stable address that survives `Vec<RegisteredHost>` reallocation.
    #[allow(dead_code)]
    module: CString,
    #[allow(dead_code)]
    symbol: CString,
    #[allow(dead_code)]
    entry: Box<HostFuncEntry>,
    #[allow(dead_code)]
    symbols: Box<[NativeSymbol; 1]>,
}

impl RuntimeInstance for WamrInstance {
    fn link_func(&mut self, module: &str, name: &str, ty: utils::FuncType, func: HostFunc) {
        let module = CString::new(module).unwrap();
        let symbol = CString::new(name).unwrap();
        let entry = Box::new(HostFuncEntry { ty, func });
        let mut symbols = Box::new([NativeSymbol {
            symbol: symbol.as_ptr(),
            func_ptr: host_trampoline as *mut c_void,
            signature: ptr::null(),
            attachment: (&*entry as *const HostFuncEntry) as *mut c_void,
        }]);
        let registered =
            unsafe { wasm_runtime_register_natives_raw(module.as_ptr(), symbols.as_mut_ptr(), 1) };
        assert!(
            registered,
            "WAMR: failed to register host function `{}::{}`",
            module.to_string_lossy(),
            symbol.to_string_lossy(),
        );
        self.hosts.push(RegisteredHost {
            module,
            symbol,
            entry,
            symbols,
        });
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let module = Module::from_vec(&self.runtime, wasm.to_vec(), "bench")
            .expect("WAMR: failed to load and validate Wasm module");
        let instance = Instance::new(&self.runtime, &module, STACK_SIZE)
            .expect("WAMR: failed to instantiate Wasm module");
        // SAFETY: `module` and `instance` borrow `self.runtime`, which outlives the returned
        // `WamrModule`: in this benchmark harness a module instance is always dropped before its
        // `WamrInstance`. Erasing the borrows lets the (self-referential) `module`/`instance` pair
        // be stored together; their FFI handles are raw pointers unaffected by the moves.
        let instance: Instance<'static> = unsafe { std::mem::transmute(instance) };
        let module: Module<'static> = unsafe { std::mem::transmute(module) };
        Box::new(WamrModule { instance, module })
    }
}

/// An instantiated WAMR module, produced by [`WamrInstance::instantiate`].
struct WamrModule {
    // `instance` is declared before `module` so it is dropped (`wasm_runtime_deinstantiate`d)
    // before the `module` it was instantiated from is unloaded.
    instance: Instance<'static>,
    #[allow(dead_code)]
    module: Module<'static>,
}

impl ModuleInstance for WamrModule {
    fn call(&mut self, name: &str, params: &[Val], results: &mut [Val]) -> anyhow::Result<()> {
        let func = Function::find_export_func(&self.instance, name)
            .map_err(|error| anyhow::anyhow!("WAMR: could not find export `{name}`: {error}"))?;
        let wasm_params: Vec<WasmValue> = params.iter().copied().map(from_utils_val).collect();
        let wasm_results = func
            .call(&self.instance, &wasm_params)
            .map_err(|error| anyhow::anyhow!("WAMR: failed to call `{name}`: {error}"))?;
        let produced = wasm_results
            .into_iter()
            .filter(|val| !matches!(val, WasmValue::Void))
            .map(into_utils_val);
        for (dst, src) in results.iter_mut().zip(produced) {
            *dst = src;
        }
        Ok(())
    }
}

/// The single WAMR *raw* native function shared by every host function linked via
/// [`WamrInstance::link_func`].
///
/// WAMR passes the arguments in `argv` with one 8-byte slot per parameter (I32/F32 in the low
/// 32 bits, I64/F64 filling the slot) and reads a single result back from slot 0. The concrete
/// host function and its signature are recovered from the native-symbol attachment.
///
/// # Safety
///
/// Registered only via [`wasm_runtime_register_natives_raw`] with the attachment pointing at a live
/// [`HostFuncEntry`], and only called by WAMR with an `argv` buffer sized for that entry's
/// signature.
extern "C" fn host_trampoline(exec_env: wasm_exec_env_t, argv: *mut u64) {
    let entry =
        unsafe { &*(wasm_runtime_get_function_attachment(exec_env) as *const HostFuncEntry) };

    let params: Vec<Val> = entry
        .ty
        .params()
        .iter()
        .enumerate()
        .map(|(i, &ty)| {
            let slot = unsafe { argv.add(i) };
            match ty {
                ValType::I32 => Val::I32(unsafe { slot.cast::<i32>().read() }),
                ValType::I64 => Val::I64(unsafe { slot.cast::<i64>().read() }),
                ValType::F32 => Val::F32(unsafe { slot.cast::<f32>().read() }),
                ValType::F64 => Val::F64(unsafe { slot.cast::<f64>().read() }),
            }
        })
        .collect();

    let mut results: Vec<Val> = entry
        .ty
        .results()
        .iter()
        .map(|&ty| Val::default_for_ty(ty))
        .collect();

    (entry.func)(&params, &mut results);

    // WAMR's raw calling convention returns at most one value, read back from slot 0.
    if let Some(&result) = results.first() {
        match result {
            Val::I32(val) => unsafe { argv.cast::<i32>().write(val) },
            Val::I64(val) => unsafe { argv.cast::<i64>().write(val) },
            Val::F32(val) => unsafe { argv.cast::<f32>().write(val) },
            Val::F64(val) => unsafe { argv.cast::<f64>().write(val) },
        }
    }
}

fn from_utils_val(val: Val) -> WasmValue {
    match val {
        Val::I32(val) => WasmValue::I32(val),
        Val::I64(val) => WasmValue::I64(val),
        Val::F32(val) => WasmValue::F32(val),
        Val::F64(val) => WasmValue::F64(val),
    }
}

fn into_utils_val(val: WasmValue) -> Val {
    match val {
        WasmValue::I32(val) => Val::I32(val),
        WasmValue::I64(val) => Val::I64(val),
        WasmValue::F32(val) => Val::F32(val),
        WasmValue::F64(val) => Val::F64(val),
        other => panic!("WAMR: unsupported result value: {other:?}"),
    }
}
