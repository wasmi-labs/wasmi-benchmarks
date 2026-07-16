#![crate_type = "dylib"]

//! Adapter for NASA's [SpaceWasm] interpreter (`github.com/nasa/spacewasm`).
//!
//! SpaceWasm is a `no_std`, Wasm 1.0 (MVP) interpreter. It has no configuration knobs, so — like
//! Stitch — the adapter exposes a single [`SpaceWasm`] runtime.
//!
//! [SpaceWasm]: https://github.com/nasa/spacewasm

use std::collections::{BTreeMap, HashSet};
use std::ops::ControlFlow;
use std::sync::{Mutex, OnceLock};

use benchmark_utils::{self as utils, StartupTestId};
use benchmark_utils::{ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, TestId};

use std::alloc::Layout;

use spacewasm::{
    AllocError, Allocator, CodeBuilder, CompilerOptions, ExportDesc, HostFunction, HostModule,
    InnerVec, Interpreter, InterpreterResult, InterpreterRunner, MemoryStatistics, Module,
    ModuleRef, Rc, Ref, Store, Value, WasmRef, WasmStream,
};
use spacewasm_util::RustSystemAllocator;

/// Backs SpaceWasm's internal collections with the process heap.
///
/// SpaceWasm is `no_std` and reaches for memory through its own C-ABI allocator hooks
/// (`__spacewasm_alloc` and friends) rather than Rust's `#[global_allocator]`. Installing this
/// allocator therefore does *not* replace the process heap and leaves the other runtimes'
/// measurements untouched. We deliberately avoid SpaceWasm's bundled `PageAllocator` (a per-page
/// bump allocator that requires LIFO release) because the benchmark harness repeatedly instantiates
/// and drops modules; a plain `malloc`/`free` handles that churn without constraints.
struct SystemAllocator;

unsafe impl Allocator for SystemAllocator {
    unsafe fn alloc(&self, layout: Layout) -> Result<*mut u8, AllocError> {
        let ptr = unsafe { std::alloc::alloc(layout) };
        if ptr.is_null() {
            Err(AllocError::AllocationFailed)
        } else {
            Ok(ptr)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { std::alloc::dealloc(ptr, layout) }
    }

    fn memory_statistics(&self) -> MemoryStatistics {
        MemoryStatistics::default()
    }
}

spacewasm::global_allocator!(SystemAllocator, SystemAllocator);

/// Number of 256-word IR pages the compiler may emit for a single module.
const MAX_CODE_PAGES: usize = 65_536;
/// Compile-time bound on control-flow nesting depth accepted by the validator.
const MAX_CONTROL_FRAMES: usize = 1024;
/// Compile-time bound on operand-stack depth accepted by the validator.
const MAX_STACK_DEPTH: usize = 4_096;
/// Runtime interpreter stack size, in 32-bit words (fits within one allocator page).
const STACK_SIZE: usize = 1 << 16;
/// Capacity of the store's Wasm module table. We only ever load a single benchmark module.
const MAX_MODULES: usize = 1;
/// Upper bound on distinct import module namespaces (`env`, `wasi_snapshot_preview1`, ...).
///
/// SpaceWasm's [`Store::new`] takes a const-generic `[HostModule; N]`, so the namespace count must
/// be resolved at compile time; [`build_store`] dispatches the runtime count onto a fixed set of
/// arms. The number of *functions* per namespace is unbounded — only the namespace count is capped.
const MAX_NAMESPACES: usize = 8;

pub struct SpaceWasm;

struct SpaceWasmInstance {
    linker: utils::Linker,
}

struct SpaceWasmModule {
    store: Store,
    /// Compiled IR text pages kept alive for the lifetime of the instance.
    text: spacewasm::Vec<spacewasm::Box<spacewasm::TextPage>>,
    /// Index of the loaded module within `store`.
    module_index: usize,
    /// Reusable parameter buffer to avoid per-call allocation.
    params: Vec<Value>,
}

impl Runtime for SpaceWasm {
    fn id(&self) -> &'static str {
        "spacewasm"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        Some(Box::new(SpaceWasmInstance {
            linker: utils::Linker::new(),
        }))
    }
}

impl SpaceWasm {
    fn can_run(&self, id: TestId) -> bool {
        // SpaceWasm only implements the Wasm 1.0 MVP, so any input relying on a later proposal
        // fails to decode/validate. The exclusions below were confirmed empirically:
        //   - `fibonacci-tail` uses the tail-call proposal (`return_call`).
        //   - `counter-param` uses multi-value block signatures (`loop (param i32) (result i32)`).
        //   - `bulk-ops` uses the bulk-memory opcodes (`0xFC` prefix).
        //   - `argon2` and the WASI-heavy modules (bz2/pulldown-cmark/spidermonkey/ffmpeg) are
        //     produced by modern toolchains and use post-MVP features (sign-extension, bulk memory,
        //     non-trapping conversions, ...).
        match id {
            TestId::Execute(id) => !matches!(
                id,
                ExecuteTestId::FibonacciTail
                    | ExecuteTestId::CounterParam
                    | ExecuteTestId::BulkOps
                    | ExecuteTestId::Sort
                    | ExecuteTestId::PrimeSieve
                    | ExecuteTestId::MatrixMultiply
                    | ExecuteTestId::Nbody
                    | ExecuteTestId::Argon2
                    | ExecuteTestId::TinyKeccak
                    | ExecuteTestId::Mandelbrot
            ),
            TestId::Startup(id) => !matches!(id, StartupTestId::Argon2,),
        }
    }
}

impl RuntimeInstance for SpaceWasmInstance {
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
        // Group the recorded host functions by import namespace, one `HostModule` per namespace.
        let mut groups: BTreeMap<&str, Vec<HostFunction>> = BTreeMap::new();
        for (module, name, ty, func) in self.linker.funcs() {
            let host_func = build_host_function(name, ty, func);
            groups.entry(module).or_default().push(host_func);
        }
        let host_modules: Vec<HostModule> = groups
            .into_iter()
            .map(|(name, functions)| HostModule {
                name: intern(name),
                globals: spacewasm::Vec::zero(),
                functions: sw_vec(functions),
                memory: spacewasm::Vec::zero(),
                table: spacewasm::Vec::zero(),
            })
            .collect();
        let mut store = build_store(host_modules);

        // Compile and validate the module.
        let mut code_builder = CodeBuilder::<MAX_CODE_PAGES>::default();
        let allocator = Rc::new(RustSystemAllocator)
            .expect("rt-spacewasm: failed to allocate Wasm memory allocator")
            .into_wasm_memory_allocator();
        let module = Module::new::<MAX_CODE_PAGES, MAX_CONTROL_FRAMES, MAX_STACK_DEPTH>(
            "benchmark-input-wasm-module",
            &mut SliceStream::new(wasm),
            &mut store,
            &mut code_builder,
            allocator,
            CompilerOptions {
                allow_memory_grow: true,
            },
        )
        .expect("rt-spacewasm: failed to compile and validate the Wasm module");
        let (text, _) = code_builder
            .finish()
            .expect("rt-spacewasm: failed to finalize compiled code");

        // Instantiate: push the module into the store and run its start section (if any).
        let module_index = {
            let mut state = store
                .allocate(STACK_SIZE)
                .expect("rt-spacewasm: failed to allocate interpreter state");
            match state.initialize_module(module, &text, usize::MAX) {
                InterpreterResult::Finished => {}
                other => panic!("rt-spacewasm: module initialization failed: {other:?}"),
            }
            state.store.modules().len() - 1
        };

        Box::new(SpaceWasmModule {
            store,
            text,
            module_index,
            params: Vec::new(),
        })
    }
}

impl ModuleInstance for SpaceWasmModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        // Resolve the exported function to a callable reference.
        let module = &self.store.modules()[self.module_index];
        let Some(export) = module.exports.iter().find(|e| &*e.name == name) else {
            anyhow::bail!("failed to find function export {name:?}")
        };
        let ExportDesc::Func(func_idx) = export.desc else {
            anyhow::bail!("export {name:?} is not a function")
        };
        let func_ref = match module.get_func_ref(func_idx) {
            Some(Ref::Module(index)) => WasmRef {
                module: ModuleRef(self.module_index as u8),
                index,
            },
            Some(Ref::Extern { module, index }) => WasmRef { module, index },
            _ => anyhow::bail!("export {name:?} does not resolve to a Wasm function"),
        };

        self.params.clear();
        self.params.extend(params.iter().copied().map(val_to_value));

        // A fresh interpreter state per call; the module's memory/globals live in the store and
        // persist across calls, matching how the other runtime adapters reuse one instance.
        let mut state = self
            .store
            .allocate(STACK_SIZE)
            .map_err(|e| anyhow::anyhow!("failed to allocate interpreter state: {e:?}"))?;
        state
            .invoke(func_ref, &self.params)
            .map_err(|e| anyhow::anyhow!("failed to invoke {name:?}: {e:?}"))?;
        let interpreter = Interpreter::default();
        let outcome = loop {
            match interpreter.run(&self.text, &mut state, usize::MAX) {
                InterpreterResult::OutOfFuel => continue,
                other => break other,
            }
        };
        match outcome {
            InterpreterResult::Finished => {}
            other => anyhow::bail!("execution of {name:?} failed: {other:?}"),
        }

        // MVP functions return at most one result.
        if let Some(result) = results.first_mut() {
            let raw = state
                .result
                .ok_or_else(|| anyhow::anyhow!("function {name:?} returned no result"))?;
            *result = value_to_val(raw.to_value(sw_val_type(result.ty())));
        }
        Ok(())
    }
}

/// Builds a SpaceWasm [`HostFunction`] that forwards to the runtime-neutral host `func` pointer.
///
/// SpaceWasm accepts a capturing `impl Fn + 'static` closure and describes signatures dynamically,
/// so this works for any signature without per-arity enumeration.
fn build_host_function(name: &str, ty: &utils::FuncType, func: utils::HostFunc) -> HostFunction {
    let name = intern(name);
    let params_sig = intern(&signature(ty.params()));
    let results_sig = intern(&signature(ty.results()));
    let result_types: Box<[utils::ValType]> = ty.results().into();
    HostFunction::new(
        name,
        params_sig.into(),
        results_sig.into(),
        move |_state, args: &[Value]| {
            let params: Vec<utils::Val> = args.iter().copied().map(value_to_val).collect();
            let mut results: Vec<utils::Val> = result_types
                .iter()
                .map(|ty| utils::Val::default_for_ty(*ty))
                .collect();
            func(&params, &mut results);
            ControlFlow::Continue(results.first().copied().map(val_to_value))
        },
    )
}

/// Constructs a [`Store`] from a runtime-determined number of host modules.
///
/// [`Store::new`] is const-generic over the module count, so the runtime count is dispatched onto a
/// fixed set of array sizes (`0..=MAX_NAMESPACES`).
fn build_store(modules: Vec<HostModule>) -> Store {
    let count = modules.len();
    let mut iter = modules.into_iter();
    macro_rules! take {
        ($n:literal) => {
            std::array::from_fn::<_, $n, _>(|_| iter.next().unwrap())
        };
    }
    let store = match count {
        0 => Store::new(MAX_MODULES, []),
        1 => Store::new(MAX_MODULES, take!(1)),
        2 => Store::new(MAX_MODULES, take!(2)),
        3 => Store::new(MAX_MODULES, take!(3)),
        4 => Store::new(MAX_MODULES, take!(4)),
        5 => Store::new(MAX_MODULES, take!(5)),
        6 => Store::new(MAX_MODULES, take!(6)),
        7 => Store::new(MAX_MODULES, take!(7)),
        8 => Store::new(MAX_MODULES, take!(8)),
        _ => panic!(
            "rt-spacewasm: more than {MAX_NAMESPACES} distinct import module namespaces are not \
             supported"
        ),
    };
    store.expect("rt-spacewasm: failed to create SpaceWasm store")
}

/// Moves items into a freshly-sized SpaceWasm [`spacewasm::Vec`].
///
/// SpaceWasm's `Vec` cannot grow past its initial capacity (`push` asserts against it), so it is
/// allocated with the exact length up front.
fn sw_vec<T>(items: Vec<T>) -> spacewasm::Vec<T> {
    let mut vec = spacewasm::Vec::new(items.len() as u32)
        .expect("rt-spacewasm: failed to allocate host-function vector");
    for item in items {
        vec.push(item);
    }
    vec
}

/// Encodes a value-type sequence as a SpaceWasm signature string (`i`/`I`/`f`/`d`).
fn signature(types: &[utils::ValType]) -> String {
    types
        .iter()
        .map(|ty| match ty {
            utils::ValType::I32 => 'i',
            utils::ValType::I64 => 'I',
            utils::ValType::F32 => 'f',
            utils::ValType::F64 => 'd',
        })
        .collect()
}

/// Interns a string, leaking each unique value once so it can be used where `&'static str` is
/// required (SpaceWasm's [`HostModule`]/[`HostFunction`] names and signature strings).
///
/// The set of import names and signatures is finite and fixed across the whole process, so this
/// leaks a bounded amount regardless of how many times a module is instantiated.
fn intern(s: &str) -> &'static str {
    static INTERNER: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut set = INTERNER
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap();
    if let Some(existing) = set.get(s) {
        return existing;
    }
    let leaked: &'static str = Box::leak(s.to_owned().into_boxed_str());
    set.insert(leaked);
    leaked
}

fn sw_val_type(ty: utils::ValType) -> spacewasm::ValType {
    match ty {
        utils::ValType::I32 => spacewasm::ValType::I32,
        utils::ValType::I64 => spacewasm::ValType::I64,
        utils::ValType::F32 => spacewasm::ValType::F32,
        utils::ValType::F64 => spacewasm::ValType::F64,
    }
}

fn val_to_value(val: utils::Val) -> Value {
    match val {
        utils::Val::I32(v) => Value::I32(v),
        utils::Val::I64(v) => Value::I64(v),
        utils::Val::F32(v) => Value::F32(v),
        utils::Val::F64(v) => Value::F64(v),
    }
}

fn value_to_val(value: Value) -> utils::Val {
    match value {
        Value::I32(v) => utils::Val::I32(v),
        Value::I64(v) => utils::Val::I64(v),
        Value::F32(v) => utils::Val::F32(v),
        Value::F64(v) => utils::Val::F64(v),
    }
}

/// A [`WasmStream`] that feeds a byte slice to SpaceWasm's chunked module reader.
struct SliceStream<'a> {
    data: &'a [u8],
    pos: usize,
    /// Owns the chunk buffers handed out as raw [`InnerVec`]s until the stream is dropped.
    bufs: Vec<Vec<u8>>,
}

impl<'a> SliceStream<'a> {
    fn new(data: &'a [u8]) -> Self {
        SliceStream {
            data,
            pos: 0,
            bufs: Vec::new(),
        }
    }
}

impl WasmStream for SliceStream<'_> {
    fn read(&mut self) -> Result<Option<InnerVec<u8>>, u8> {
        if self.pos >= self.data.len() {
            return Ok(None);
        }
        let end = (self.pos + 1024).min(self.data.len());
        let mut buf = self.data[self.pos..end].to_vec();
        self.pos = end;
        let chunk = InnerVec {
            ptr: buf.as_mut_ptr(),
            capacity: buf.capacity() as u32,
            len: buf.len() as u32,
        };
        self.bufs.push(buf);
        Ok(Some(chunk))
    }

    fn return_(&mut self, _chunk: InnerVec<u8>) {}
}
