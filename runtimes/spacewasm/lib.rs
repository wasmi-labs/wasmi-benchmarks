#![crate_type = "dylib"]

//! Adapter for NASA's [SpaceWasm] interpreter (`github.com/nasa/spacewasm`).
//!
//! SpaceWasm is a `no_std`, Wasm 1.0 (MVP) interpreter. It has no configuration knobs, so — like
//! Stitch — the adapter exposes a single [`SpaceWasm`] runtime.
//!
//! [SpaceWasm]: https://github.com/nasa/spacewasm

use benchmark_utils::{self as utils};
use benchmark_utils::{ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, TestId};
use spacewasm::{
    AllocError, Allocator, CodeBuilder, CompilerOptions, Engine, ExportDesc,
    HOST_FUNCTION_NAME_CAP, HOST_MODULE_NAME_CAP, HostFunction, HostModule, HostName, HostValList,
    InnerVec, Interpreter, InterpreterResult, InterpreterRunner, Memory, MemoryKind,
    MemoryStatistics, Module, ModuleRef, Rc, Ref, StartInvocation, Value, WasmMemoryAllocator,
    WasmRef, WasmStream,
};
use std::alloc::Layout;
use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::ptr::NonNull;

/// Backs SpaceWasm's internal collections with the process heap.
///
/// SpaceWasm is `no_std` and reaches for memory through its own C-ABI allocator hooks
/// (`__spacewasm_alloc` and friends) rather than Rust's `#[global_allocator]`. Installing this
/// allocator therefore does *not* replace the process heap and leaves the other runtimes'
/// measurements untouched. We deliberately avoid SpaceWasm's bundled `PageAllocator` (a per-page
/// bump allocator that requires LIFO release) because the benchmark harness repeatedly instantiates
/// and drops modules; a plain `malloc`/`free` handles that churn without constraints.
///
/// The same type also serves as the [`WasmMemoryAllocator`] backing Wasm linear memories. SpaceWasm
/// ships such an implementation only behind `#[cfg(test)]` (`spacewasm::test_support`), so
/// embedders supply their own — upstream's spec-test harness does exactly this.
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

impl WasmMemoryAllocator for SystemAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        unsafe { NonNull::new(std::alloc::alloc(layout)).ok_or(AllocError::AllocationFailed) }
    }

    fn reallocate(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        layout: Layout,
    ) -> Result<NonNull<u8>, AllocError> {
        unsafe {
            NonNull::new(std::alloc::realloc(ptr.as_ptr(), old_layout, layout.size()))
                .ok_or(AllocError::AllocationFailed)
        }
    }

    fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) }
    }
}

spacewasm::global_allocator!(SystemAllocator, SystemAllocator);

/// Number of 256-word IR pages the compiler may emit for a single module.
const MAX_CODE_PAGES: u32 = 65_536;
/// Compile-time bound on control-flow nesting depth accepted by the validator.
const MAX_CONTROL_FRAMES: usize = 1024;
/// Compile-time bound on operand-stack depth accepted by the validator.
const MAX_STACK_DEPTH: usize = 4_096;
/// Runtime interpreter stack size, in 32-bit words (fits within one allocator page).
const STACK_SIZE: usize = 1 << 16;
/// Capacity of the store's Wasm module table. We only ever load a single benchmark module.
const MAX_MODULES: usize = 1;

pub struct SpaceWasm;

struct SpaceWasmInstance {
    linker: utils::Linker,
}

struct SpaceWasmModule {
    /// Owns the store (modules, host modules) and the interpreter stack.
    engine: Engine,
    /// Owns the compiled IR text pages; kept alive for the lifetime of the instance because the
    /// interpreter is handed them on every run.
    code_builder: CodeBuilder,
    /// Index of the loaded module within the engine's store.
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
        !matches!(
            id,
            TestId::Execute(
                ExecuteTestId::FibonacciTail | ExecuteTestId::CounterParam | ExecuteTestId::BulkOps
            )
        )
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
                name: host_name(name, "import module namespace", HOST_MODULE_NAME_CAP),
                globals: spacewasm::Vec::zero(),
                functions: sw_vec(functions),
                memory: spacewasm::Vec::zero(),
                table: spacewasm::Vec::zero(),
            })
            .collect();
        // The engine owns the store and a single interpreter stack shared by all calls.
        let mut engine = Engine::new(STACK_SIZE, MAX_MODULES, sw_vec(host_modules))
            .expect("rt-spacewasm: failed to create SpaceWasm engine");

        // Compile and validate the module.
        let mut code_builder = CodeBuilder::new(CompilerOptions {
            allow_memory_grow: true,
            // `0` disables the control-flow backpatch iteration limit, which would otherwise reject
            // valid programs with deeply nested control flow.
            max_backpatch_iterations: 0,
            max_code_pages: MAX_CODE_PAGES,
        })
        .expect("rt-spacewasm: failed to allocate the IR code builder");
        let allocator = Rc::new(SystemAllocator)
            .expect("rt-spacewasm: failed to allocate Wasm memory allocator")
            .into_wasm_memory_allocator();
        let module = Module::new::<MAX_CONTROL_FRAMES, MAX_STACK_DEPTH>(
            "benchmark-input-wasm-module",
            &mut SliceStream::new(wasm),
            &mut engine.store,
            &mut code_builder,
            allocator,
        )
        .expect("rt-spacewasm: failed to compile and validate the Wasm module");

        // Instantiate: push the module into the store and run its start section (if any).
        let module_ref = engine
            .push_module(module)
            .expect("rt-spacewasm: failed to push the module into the engine store");
        match engine.invoke_start(module_ref) {
            StartInvocation::Finished => {}
            // A Wasm start function is only seeded by `invoke_start`; the interpreter drives it.
            StartInvocation::Running => match run_to_completion(&code_builder, &mut engine) {
                InterpreterResult::Finished => {}
                other => panic!("rt-spacewasm: module initialization failed: {other:?}"),
            },
            other => panic!("rt-spacewasm: module initialization failed: {other:?}"),
        }

        Box::new(SpaceWasmModule {
            engine,
            code_builder,
            module_index: module_ref.0 as usize,
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
        let module = &self.engine.store.modules()[self.module_index];
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

        // The engine — and with it the interpreter stack — is reused across calls, so the run state
        // is rewound before each invocation; `Engine::invoke` requires an idle engine and a
        // previously trapped call would leave the stack/program counter dirty. The module's memory
        // and globals live in the store and persist, matching the other runtime adapters.
        self.engine.reset();
        self.engine
            .invoke(func_ref, &self.params)
            .map_err(|e| anyhow::anyhow!("failed to invoke {name:?}: {e:?}"))?;
        match run_to_completion(&self.code_builder, &mut self.engine) {
            InterpreterResult::Finished => {}
            other => anyhow::bail!("execution of {name:?} failed: {other:?}"),
        }

        // MVP functions return at most one result.
        if let Some(result) = results.first_mut() {
            let raw = self
                .engine
                .result
                .ok_or_else(|| anyhow::anyhow!("function {name:?} returned no result"))?;
            *result = value_to_val(raw.to_value(sw_val_type(result.ty())));
        }
        Ok(())
    }

    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()> {
        let bytes = self
            .memory(name)?
            .load(ptr as usize, buffer.len())
            .map_err(|e| anyhow::anyhow!("failed to read memory {name:?}: {e:?}"))?;
        buffer.copy_from_slice(bytes);
        Ok(())
    }

    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()> {
        self.memory(name)?
            .store(ptr as usize, buffer)
            .map_err(|e| anyhow::anyhow!("failed to write memory {name:?}: {e:?}"))?;
        Ok(())
    }
}

impl SpaceWasmModule {
    /// Resolves the memory exported by `name` to its backing [`Memory`].
    ///
    /// SpaceWasm has no memory-by-name accessor, so this mirrors how [`SpaceWasmModule::call`]
    /// resolves an exported function: find the export, confirm it is a memory, then follow the
    /// module's [`MemoryKind`] to the owning [`Memory`]. Both [`Memory::load`] and [`Memory::store`]
    /// take `&self`, so this shared `&self` resolver serves reads and writes alike.
    fn memory(&self, name: &str) -> anyhow::Result<&Rc<Memory>> {
        let module = &self.engine.store.modules()[self.module_index];
        let Some(export) = module.exports.iter().find(|e| &*e.name == name) else {
            anyhow::bail!("failed to find memory export {name:?}")
        };
        let ExportDesc::Mem(_) = export.desc else {
            anyhow::bail!("export {name:?} is not a memory")
        };
        match &module.memory {
            Some(MemoryKind::Owned(mem)) => Ok(mem),
            Some(MemoryKind::Import(module_ref)) => {
                match &self.engine.store.modules()[module_ref.0 as usize].memory {
                    Some(MemoryKind::Owned(mem)) => Ok(mem),
                    _ => {
                        anyhow::bail!(
                            "imported memory {name:?} does not resolve to an owned memory"
                        )
                    }
                }
            }
            Some(MemoryKind::ImportHost(_)) => {
                anyhow::bail!("host-imported memory {name:?} is not supported")
            }
            None => anyhow::bail!("module has no memory for export {name:?}"),
        }
    }
}

/// Builds a SpaceWasm [`HostFunction`] that forwards to the runtime-neutral host `func` pointer.
///
/// SpaceWasm accepts a capturing `impl Fn + 'static` closure and describes signatures dynamically,
/// so this works for any signature without per-arity enumeration.
fn build_host_function(name: &str, ty: &utils::FuncType, func: utils::HostFunc) -> HostFunction {
    let host_name = host_name(name, "import function name", HOST_FUNCTION_NAME_CAP);
    let params = host_val_list(name, ty.params());
    let results = host_val_list(name, ty.results());
    let result_types: Box<[utils::ValType]> = ty.results().into();
    HostFunction::try_new(
        host_name,
        params,
        results,
        move |_engine: &mut Engine, args: &[Value]| {
            let params: Vec<utils::Val> = args.iter().copied().map(value_to_val).collect();
            let mut results: Vec<utils::Val> = result_types
                .iter()
                .map(|ty| utils::Val::default_for_ty(*ty))
                .collect();
            func(&params, &mut results);
            ControlFlow::Continue(results.first().copied().map(val_to_value))
        },
    )
    .unwrap_or_else(|e| panic!("rt-spacewasm: unsupported signature for import {name:?}: {e:?}"))
}

/// Runs the interpreter until it stops for a reason other than exhausting its instruction budget.
///
/// The budget is already `usize::MAX`, so the retry only guards against a bounded run slipping
/// through; every other outcome (finished, trap, pause) is returned to the caller.
fn run_to_completion(code_builder: &CodeBuilder, engine: &mut Engine) -> InterpreterResult {
    loop {
        match Interpreter.run(code_builder.pages(), engine, usize::MAX) {
            InterpreterResult::OutOfFuel => continue,
            other => break other,
        }
    }
}

/// Converts `name` into SpaceWasm's inline, fixed-capacity host name representation.
///
/// `what` and `cap` only shape the panic message; SpaceWasm caps module and function names at
/// [`HOST_MODULE_NAME_CAP`] / [`HOST_FUNCTION_NAME_CAP`] bytes respectively.
fn host_name<const CAPACITY: usize>(name: &str, what: &str, cap: usize) -> HostName<CAPACITY> {
    HostName::try_from_str(name).unwrap_or_else(|_| {
        panic!("rt-spacewasm: {what} {name:?} exceeds SpaceWasm's {cap}-byte limit")
    })
}

/// Converts a value-type sequence into SpaceWasm's inline host signature representation.
fn host_val_list(name: &str, types: &[utils::ValType]) -> HostValList {
    HostValList::try_new(&signature(types)).unwrap_or_else(|e| {
        panic!("rt-spacewasm: unsupported signature for import {name:?}: {e:?}")
    })
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
