#![crate_type = "dylib"]

mod linker;
mod val;

pub use self::linker::{HostFunc, Linker};
pub use self::val::{FuncType, Val, ValType};
use core::fmt;

/// A WebAssembly runtime description.
///
/// Represents a Wasm runtime with a specific configuration.
pub trait Runtime {
    /// Returns the unique ID of the Wasm runtime and its configuration as string.
    fn id(&self) -> &'static str;

    /// Compiles the `wasm` using the Wasm runtime and its configuration.
    ///
    /// # Returns
    ///
    /// - Returns `true` if `self` can compile the test with identity `id`.
    /// - Returns `false` otherwise.
    ///
    /// # Note
    ///
    /// This is a temporary method used by the compilation benchmarks. It will be
    /// removed once those are migrated to instantiation benchmarks that go through
    /// [`Runtime::setup`] and [`RuntimeInstance::load`] like the execution benchmarks.
    fn compile(&self, id: CompileTestId, wasm: &[u8]) -> bool;

    /// Sets up and returns a [`RuntimeInstance`] if `self` can run `id`.
    ///
    /// Otherwise returns `None`.
    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>>;
}

/// A concrete instance of a WebAssembly (Wasm) runtime.
pub trait RuntimeInstance {
    /// Defines the host `func` with signature `ty` in the runtime's linker under `module::name`.
    ///
    /// # Note
    ///
    /// Must be called before [`Self::load`].
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: FuncType,
        func: fn(params: &[Val], results: &mut [Val]),
    );

    /// Consumes `self` to instantiate the `wasm` module with previously linked functions.
    fn instantiate(self: Box<Self>, wasm: &[u8]) -> Box<dyn ModuleInstance>;
}

/// A module instance of a WebAssembly (Wasm) runtime.
pub trait ModuleInstance {
    /// Calls the function exported by `name` with `params` and writes the results back into `results`.
    ///
    /// # Note
    ///
    /// It is the callers responsibility to provide `params` and `results` buffers big enough to satisfy the called function.
    fn call(&mut self, name: &str, params: &[Val], results: &mut [Val]) -> anyhow::Result<()>;
}

/// A Wasm runtime that is capable of being benchmarked.
pub trait BenchRuntime {
    /// Returns the unique ID of the Wasm runtime and its configuration as string.
    fn id(&self) -> &'static str;

    /// Returns `true` if `self` can run the test with the given `id`.
    fn can_run(&self, id: TestId) -> bool;

    /// Compiles the `wasm` using the Wasm runtime and its configuration.
    fn compile(&self, wasm: &[u8]);

    /// Loads a Wasm module instance using the Wasm runtime and its configuration.
    ///
    /// The returned Wasm module instance can then be used to issue calls.
    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance>;

    /// Runs the given Coremark Wasm test and returns the result.
    fn coremark(&self, wasm: &[u8], elapsed_ms: fn() -> u32) -> f32;
}

/// The module instance of a Wasm runtime that is capable of being benchmarked.
pub trait BenchInstance {
    /// Calls the function exported by `name` with `params` and writes the results back into `results`.
    ///
    /// # Note
    ///
    /// It is the callers responsibility to provide `params` and `results` buffers big enough to satisfy the called function.
    fn call(&mut self, name: &str, params: &[Val], results: &mut [Val]) -> anyhow::Result<()>;
}

#[derive(Copy, Clone)]
pub enum TestId {
    Compile(CompileTestId),
    Execute(ExecuteTestId),
}

impl From<CompileTestId> for TestId {
    fn from(value: CompileTestId) -> Self {
        Self::Compile(value)
    }
}

impl From<ExecuteTestId> for TestId {
    fn from(value: ExecuteTestId) -> Self {
        Self::Execute(value)
    }
}

#[derive(Copy, Clone)]
pub enum CompileTestId {
    Erc20,
    Bz2,
    PulldownCmark,
    Spidermonkey,
    Ffmpeg,
    CoreMark,
    Argon2,
}

#[derive(Copy, Clone)]
pub enum ExecuteTestId {
    CounterLocal,
    CounterParam,
    FibonacciIter,
    FibonacciRec,
    FibonacciTail,
    Primes,
    MatrixMultiply,
    Argon2,
    BulkOps,
    CoreMark,
}

/// Converts the `.wat` encoded `bytes` into `.wasm` encoded bytes.
pub fn wat2wasm(bytes: &[u8]) -> Vec<u8> {
    wat::parse_bytes(bytes).unwrap().into_owned()
}

/// The encoded format of the input.
#[derive(Debug, Copy, Clone)]
pub enum InputEncoding {
    /// The input is encoded as `.wat` text format.
    Wat,
    /// The input is encoded as `.wasm` binary.
    Wasm,
}

impl fmt::Display for InputEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputEncoding::Wat => "wat".fmt(f),
            InputEncoding::Wasm => "wasm".fmt(f),
        }
    }
}

/// Returns the bytes of the named benchmark file with the given `encoding`.
///
/// # Panics
///
/// - If the file cannot be found at the associated path.
/// - If the file contents cannot be decoded as either `.wat` or `.wasm`.
/// - If the `.wat` file format cannot be encoded into the `.wasm` format.
pub fn read_benchmark_file(encoding: InputEncoding, name: &str) -> Vec<u8> {
    let path = format!("benches/res/{encoding}/{name}.{encoding}");
    let wasm_or_wat = std::fs::read(&path).unwrap_or_else(|error| {
        panic!("failed to read benchmark input:\n\tpath = {path}\n\terror = {error}")
    });
    match encoding {
        InputEncoding::Wasm => wasm_or_wat,
        InputEncoding::Wat => wat::parse_bytes(&wasm_or_wat[..])
            .unwrap_or_else(|error| panic!("failed to convert `.wat` to `.wasm`: {error}"))
            .to_vec(),
    }
}
