#![crate_type = "dylib"]

mod id;
mod linker;
mod typed_call;
mod val;

pub use self::id::{ExecuteTestId, StartupTestId, TestId};
pub use self::linker::{HostFunc, Linker};
pub use self::typed_call::{CallTyped, WasmParams, WasmResults, WasmValue};
pub use self::val::{FuncType, TypeMismatch, Val, ValType};
use core::fmt;
use std::fs;

/// A WebAssembly runtime description.
///
/// Represents a Wasm runtime with a specific configuration.
pub trait Runtime {
    /// Returns the unique ID of the Wasm runtime and its configuration as string.
    fn id(&self) -> &'static str;

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
    /// Must be called before [`Self::instantiate`].
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: FuncType,
        func: fn(params: &[Val], results: &mut [Val]),
    );

    /// Instantiates the `wasm` module with previously linked functions.
    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance>;
}

/// A module instance of a WebAssembly (Wasm) runtime.
pub trait ModuleInstance {
    /// Calls the function exported by `name` with `params` and writes the results back into `results`.
    ///
    /// # Note
    ///
    /// It is the callers responsibility to provide `params` and `results` buffers big enough to satisfy the called function.
    fn call(&mut self, name: &str, params: &[Val], results: &mut [Val]) -> anyhow::Result<()>;

    /// Reads from the memory exported from `self` by `name`: `buffer = memory[ptr..ptr+buffer.len()]`
    fn read_memory(&mut self, name: &str, ptr: u32, buffer: &mut [u8]) -> anyhow::Result<()>;

    /// Writes to the memory export from `self` by `name`: `memory[ptr..ptr+buffer.len()] = buffer`
    fn write_memory(&mut self, name: &str, ptr: u32, buffer: &[u8]) -> anyhow::Result<()>;
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
    /// The input is a `.wasm` binary compiled from Rust.
    RustCompiledWasm,
}

impl fmt::Display for InputEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wat => "wat".fmt(f),
            Self::Wasm => "wasm".fmt(f),
            Self::RustCompiledWasm => "rust".fmt(f),
        }
    }
}

/// Returns the `.wasm` bytes of the benchmark file for `id` with `encoding`.
///
/// # Panics
///
/// - If the file cannot be found at the associated path.
/// - If the file contents cannot be decoded as either `.wat` or `.wasm`.
/// - If the `.wat` file format cannot be encoded into the `.wasm` format.
pub fn read_benchmark_file(encoding: InputEncoding, id: TestId) -> Vec<u8> {
    let wasm_or_wat = fetch_benchmark_file(encoding, id);
    match encoding {
        InputEncoding::Wasm => wasm_or_wat,
        InputEncoding::Wat | InputEncoding::RustCompiledWasm => wat::parse_bytes(&wasm_or_wat[..])
            .unwrap_or_else(|error| panic!("failed to convert `.wat` to `.wasm`: {error}"))
            .to_vec(),
    }
}

/// Fetches the benchmark `.wat` or `.wasm` file for `id` with `encoding`.
fn fetch_benchmark_file(encoding: InputEncoding, id: TestId) -> Vec<u8> {
    let path = match encoding {
        InputEncoding::Wat | InputEncoding::Wasm => {
            format!("res/{encoding}/{id}.{encoding}")
        }
        InputEncoding::RustCompiledWasm => {
            format!("res/{encoding}/cases/{id}/out.wasm")
        }
    };
    fs::read(&path).unwrap_or_else(|error| {
        panic!("failed to fetch benchmark input:\n\tpath = {path}\n\terror = {error}")
    })
}
