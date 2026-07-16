#![crate_type = "dylib"]

use benchmark_utils::{self as utils, HostFunc};
use benchmark_utils::{ModuleInstance, Runtime, RuntimeInstance, TestId};
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::Once;

pub struct V8;

/// A configured V8 runtime with its recorded host functions, produced by [`V8::setup`].
///
/// V8 binds host functions to an [`Isolate`](v8::Isolate)/[`Context`](v8::Context), so rather than
/// holding a live isolate the recorded host functions are replayed into a fresh isolate on every
/// instantiation (mirroring the Tinywasm adapter).
struct V8Instance {
    linker: utils::Linker,
}

/// An instantiated V8 module, produced by [`V8Instance::instantiate`].
///
/// Everything V8 is bound to an isolate. Opening any V8 scope requires `&mut Isolate`, but
/// [`ModuleInstance::read_memory`] only has `&self`, so the isolate lives behind a [`RefCell`]
/// (single-threaded, non-reentrant, so the runtime borrow never panics).
struct V8Module {
    isolate: RefCell<v8::OwnedIsolate>,
    /// The instantiation context, kept alive as a [`Global`](v8::Global) so scopes can be reopened.
    context: v8::Global<v8::Context>,
    /// The module instance's `exports` object.
    exports: v8::Global<v8::Object>,
    /// Backing storage for the host functions referenced (by raw pointer) from the import object.
    /// Declared last so it is dropped after `isolate` (fields drop top-to-bottom); the pointers
    /// must stay valid for the whole lifetime of the isolate. The `Vec`'s heap buffer keeps a
    /// stable address when the `Vec` is moved into this struct, so pointers taken into it during
    /// [`instantiate`](V8Instance::instantiate) remain valid.
    _host_data: Vec<HostFuncData>,
}

/// A recorded host function plus its signature, referenced from a V8 import function via a raw
/// [`v8::External`] pointer (V8 callbacks must be plain `fn`s and cannot capture state directly).
struct HostFuncData {
    func: HostFunc,
    ty: utils::FuncType,
}

impl Runtime for V8 {
    fn id(&self) -> &'static str {
        "v8"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        init_v8();
        Some(Box::new(V8Instance {
            linker: utils::Linker::new(),
        }))
    }
}

impl V8 {
    fn can_run(&self, _id: TestId) -> bool {
        // V8 supports every proposal exercised by the benchmark suite (tail calls, bulk memory,
        // multi-value, SIMD) and instantiates the WASI command modules with inert import stubs.
        true
    }
}

/// Initializes the V8 platform exactly once for the whole process.
fn init_v8() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

impl RuntimeInstance for V8Instance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        self.linker.define(module, name, ty, func);
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        // Record the host functions before entering any scope. `host_data` is built once here and
        // never grows again, so after this loop its elements have stable addresses that the import
        // functions can reference by raw pointer (the addresses survive moving the `Vec` into the
        // returned `V8Module`). `names` is transient and only used to build the import object.
        let mut host_data: Vec<HostFuncData> = Vec::new();
        let mut names: Vec<(String, String)> = Vec::new();
        for (module, name, ty, func) in self.linker.funcs() {
            host_data.push(HostFuncData {
                func,
                ty: ty.clone(),
            });
            names.push((module.to_string(), name.to_string()));
        }

        let mut isolate = v8::Isolate::new(Default::default());
        let (context, exports) = {
            v8::scope!(let handle_scope, &mut isolate);
            let context = v8::Context::new(handle_scope, Default::default());
            let scope = &mut v8::ContextScope::new(handle_scope, context);

            // Build the JS import object `{ module: { name: <fn>, .. }, .. }`.
            let imports = v8::Object::new(scope);
            for (data, (module, name)) in host_data.iter().zip(&names) {
                let external =
                    v8::External::new(scope, (data as *const HostFuncData) as *mut c_void);
                let func = v8::Function::builder(host_trampoline)
                    .data(external.into())
                    .build(scope)
                    .expect("v8: failed to create host function");
                let module_key = v8::String::new(scope, module).unwrap();
                let module_obj = match imports.get(scope, module_key.into()) {
                    Some(value) if value.is_object() => value.to_object(scope).unwrap(),
                    _ => {
                        let obj = v8::Object::new(scope);
                        let _ = imports.set(scope, module_key.into(), obj.into());
                        obj
                    }
                };
                let name_key = v8::String::new(scope, name).unwrap();
                let _ = module_obj.set(scope, name_key.into(), func.into());
            }

            // Compile the module and instantiate it via `new WebAssembly.Instance(module, imports)`.
            let module = v8::WasmModuleObject::compile(scope, wasm)
                .expect("v8: failed to compile Wasm module");
            let instance_ctor = webassembly_instance_ctor(scope, context);
            let args = [module.into(), imports.into()];
            let instance = instance_ctor
                .new_instance(scope, &args)
                .expect("v8: failed to instantiate Wasm module");
            let exports_key = v8::String::new(scope, "exports").unwrap();
            let exports = instance
                .get(scope, exports_key.into())
                .unwrap()
                .to_object(scope)
                .unwrap();

            (
                v8::Global::new(scope, context),
                v8::Global::new(scope, exports),
            )
        };

        Box::new(V8Module {
            isolate: RefCell::new(isolate),
            context,
            exports,
            _host_data: host_data,
        })
    }
}

impl ModuleInstance for V8Module {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let mut isolate = self.isolate.borrow_mut();
        v8::scope_with_context!(let scope, &mut *isolate, &self.context);
        let exports = v8::Local::new(scope, &self.exports);
        let name_key = v8::String::new(scope, name).unwrap();
        let func = exports
            .get(scope, name_key.into())
            .ok_or_else(|| anyhow::anyhow!("v8: missing export `{name}`"))?;
        let func = v8::Local::<v8::Function>::try_from(func)
            .map_err(|_| anyhow::anyhow!("v8: export `{name}` is not a function"))?;
        let args: Vec<v8::Local<v8::Value>> = params
            .iter()
            .map(|param| val_to_js(scope, *param))
            .collect();
        let recv = v8::undefined(scope).into();
        let ret = func
            .call(scope, recv, &args)
            .ok_or_else(|| anyhow::anyhow!("v8: call to `{name}` failed"))?;
        write_results(scope, results, ret);
        Ok(())
    }

    fn read_memory(&self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let mut isolate = self.isolate.borrow_mut();
        v8::scope_with_context!(let scope, &mut *isolate, &self.context);
        let array_buffer = memory_buffer(scope, &self.exports, name)?;
        let store = array_buffer.get_backing_store();
        let (base, len) = backing_store_ptr(&store);
        let start = ptr as usize;
        let end = start
            .checked_add(buffer.len())
            .filter(|&end| end <= len)
            .ok_or_else(|| anyhow::anyhow!("v8: out-of-bounds read from memory `{name}`"))?;
        // SAFETY: `base` points at `len` valid bytes for the lifetime of `store`, and the range
        // `start..end` was just bounds-checked against `len`.
        unsafe {
            std::ptr::copy_nonoverlapping(base.add(start), buffer.as_mut_ptr(), end - start);
        }
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        let mut isolate = self.isolate.borrow_mut();
        v8::scope_with_context!(let scope, &mut *isolate, &self.context);
        let array_buffer = memory_buffer(scope, &self.exports, name)?;
        let store = array_buffer.get_backing_store();
        let (base, len) = backing_store_ptr(&store);
        let start = ptr as usize;
        let end = start
            .checked_add(buffer.len())
            .filter(|&end| end <= len)
            .ok_or_else(|| anyhow::anyhow!("v8: out-of-bounds write to memory `{name}`"))?;
        // SAFETY: `base` points at `len` valid bytes for the lifetime of `store`, and the range
        // `start..end` was just bounds-checked against `len`.
        unsafe {
            std::ptr::copy_nonoverlapping(buffer.as_ptr(), base.add(start), end - start);
        }
        Ok(())
    }
}

/// Returns the `WebAssembly.Instance` constructor from the context's global object.
fn webassembly_instance_ctor<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    context: v8::Local<v8::Context>,
) -> v8::Local<'s, v8::Function> {
    let global = context.global(scope);
    let webassembly_key = v8::String::new(scope, "WebAssembly").unwrap();
    let webassembly = global
        .get(scope, webassembly_key.into())
        .unwrap()
        .to_object(scope)
        .unwrap();
    let instance_key = v8::String::new(scope, "Instance").unwrap();
    let instance = webassembly.get(scope, instance_key.into()).unwrap();
    v8::Local::<v8::Function>::try_from(instance).unwrap()
}

/// Resolves the exported `WebAssembly.Memory`'s backing `ArrayBuffer` for `name`.
fn memory_buffer<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    exports: &v8::Global<v8::Object>,
    name: &str,
) -> anyhow::Result<v8::Local<'s, v8::ArrayBuffer>> {
    let exports = v8::Local::new(scope, exports);
    let name_key = v8::String::new(scope, name).unwrap();
    let memory = exports
        .get(scope, name_key.into())
        .and_then(|value| value.to_object(scope))
        .ok_or_else(|| anyhow::anyhow!("v8: missing memory export `{name}`"))?;
    let buffer_key = v8::String::new(scope, "buffer").unwrap();
    let buffer = memory
        .get(scope, buffer_key.into())
        .ok_or_else(|| anyhow::anyhow!("v8: memory export `{name}` has no `buffer`"))?;
    v8::Local::<v8::ArrayBuffer>::try_from(buffer)
        .map_err(|_| anyhow::anyhow!("v8: memory export `{name}` is not a `WebAssembly.Memory`"))
}

/// Returns the base pointer and byte length of an `ArrayBuffer`'s backing store.
fn backing_store_ptr(store: &v8::SharedRef<v8::BackingStore>) -> (*mut u8, usize) {
    let len = store.byte_length();
    let base = store
        .data()
        .map(|data| data.as_ptr() as *mut u8)
        .unwrap_or(std::ptr::NonNull::<u8>::dangling().as_ptr());
    (base, len)
}

/// The single V8 callback shared by all linked host functions; the concrete [`HostFuncData`] is
/// recovered from the function's [`v8::External`] data slot.
fn host_trampoline(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let external = v8::Local::<v8::External>::try_from(args.data())
        .expect("v8: host function data is not an External");
    // SAFETY: the pointer was created from a `Box<HostFuncData>` that is kept alive in the owning
    // `V8Module` for at least as long as this isolate (and thus this callback) can run.
    let data = unsafe { &*(external.value() as *const HostFuncData) };

    let params: Vec<utils::Val> = data
        .ty
        .params()
        .iter()
        .enumerate()
        .map(|(i, ty)| js_to_val(scope, *ty, args.get(i as i32)))
        .collect();
    let mut results: Vec<utils::Val> = data
        .ty
        .results()
        .iter()
        .copied()
        .map(utils::Val::default_for_ty)
        .collect();
    (data.func)(&params, &mut results);

    match results.as_slice() {
        [] => {}
        [result] => rv.set(val_to_js(scope, *result)),
        results => {
            let array = v8::Array::new(scope, results.len() as i32);
            for (i, result) in results.iter().enumerate() {
                let value = val_to_js(scope, *result);
                let _ = array.set_index(scope, i as u32, value);
            }
            rv.set(array.into());
        }
    }
}

/// Writes the JS return `value` of an exported call back into `results` using each slot's type.
fn write_results(
    scope: &mut v8::PinScope,
    results: &mut [utils::Val],
    value: v8::Local<v8::Value>,
) {
    match results {
        [] => {}
        [result] => *result = js_to_val(scope, result.ty(), value),
        results => {
            let array = v8::Local::<v8::Array>::try_from(value)
                .expect("v8: expected an array for multi-value result");
            for (i, result) in results.iter_mut().enumerate() {
                let element = array.get_index(scope, i as u32).unwrap();
                *result = js_to_val(scope, result.ty(), element);
            }
        }
    }
}

/// Converts a benchmark [`Val`](utils::Val) into a JS value (`i64` maps to `BigInt`).
fn val_to_js<'s>(scope: &mut v8::PinScope<'s, '_>, val: utils::Val) -> v8::Local<'s, v8::Value> {
    match val {
        utils::Val::I32(val) => v8::Integer::new(scope, val).into(),
        utils::Val::I64(val) => v8::BigInt::new_from_i64(scope, val).into(),
        utils::Val::F32(val) => v8::Number::new(scope, val as f64).into(),
        utils::Val::F64(val) => v8::Number::new(scope, val).into(),
    }
}

/// Converts a JS value into a benchmark [`Val`](utils::Val) of the given type (`i64` from `BigInt`).
fn js_to_val(
    scope: &mut v8::PinScope,
    ty: utils::ValType,
    value: v8::Local<v8::Value>,
) -> utils::Val {
    match ty {
        utils::ValType::I32 => utils::Val::I32(value.int32_value(scope).unwrap()),
        utils::ValType::I64 => {
            let big =
                v8::Local::<v8::BigInt>::try_from(value).expect("v8: expected a BigInt for i64");
            utils::Val::I64(big.i64_value().0)
        }
        utils::ValType::F32 => utils::Val::F32(value.number_value(scope).unwrap() as f32),
        utils::ValType::F64 => utils::Val::F64(value.number_value(scope).unwrap()),
    }
}
