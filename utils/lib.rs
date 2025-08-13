#![crate_type = "dylib"]

use std::fmt;
pub use wasmi_new::ModuleImportsIter;
pub use wasmi_new::{ExternType, Module, ValType};

/// A Wasm runtime that is capable of being benchmarked.
pub trait BenchRuntime {
    /// Returns the name of the Wasm runtime and its configuration.
    fn name(&self) -> &'static str;

    /// Returns the [`TestFilter`] which applies to the Wasm runtime and its configuration.
    fn test_filter(&self) -> TestFilter {
        TestFilter::default()
    }

    /// Compiles the `wasm` using the Wasm runtime and its configuration.
    fn compile(&self, wasm: &[u8], imports: ModuleImportsIter);

    /// Loads a Wasm module instance using the Wasm runtime and its configuration.
    ///
    /// The returned Wasm module instance can then be used to issue calls.
    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance>;

    /// Runs the given Coremark Wasm test and returns the result.
    fn coremark(&self, wasm: &[u8]) -> f32;
}

/// The module instance of a Wasm runtime that is capable of being benchmarked.
pub trait BenchInstance {
    /// Calls the callable Wasm runtime module instance.
    fn call(&mut self, input: i64);
}

/// Used to query elapsed time since last time this has been called. Used for Coremark benchmark.
pub fn elapsed_ms() -> u32 {
    use std::time::Instant;
    static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let elapsed = STARTED.get_or_init(Instant::now).elapsed();
    elapsed.as_millis() as u32
}

/// Parses the `wasm` bytes and returns a Wasmi [`Module`].
///
/// The returned [`Module`] can then be used to query import information.
/// This import information is then fed into the benchmarked VMs for their disposal.
///
/// [`Module`]: wasmi_new::Module
pub fn parse_module(wasm: &[u8]) -> wasmi_new::Module {
    let mut config = wasmi_new::Config::default();
    config.compilation_mode(wasmi_new::CompilationMode::Lazy);
    let engine = wasmi_new::Engine::new(&config);
    wasmi_new::Module::new(&engine, wasm).unwrap()
}

#[derive(Debug, Copy, Clone)]
pub struct TestFilter {
    pub execute: ExecuteTestFilter,
    pub compile: CompileTestFilter,
}

impl TestFilter {
    pub fn set_to(flag: bool) -> Self {
        Self {
            execute: ExecuteTestFilter::set_to(flag),
            compile: CompileTestFilter::set_to(flag),
        }
    }
}

impl Default for TestFilter {
    fn default() -> Self {
        Self::set_to(true)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExecuteTestFilter {
    pub counter: bool,
    pub fib_iterative: bool,
    pub fib_recursive: bool,
    pub fib_tailrec: bool,
    pub primes: bool,
    pub matrix_multiply: bool,
    pub argon2: bool,
    pub bulk_ops: bool,
    pub coremark: bool,
}

impl ExecuteTestFilter {
    pub fn set_to(flag: bool) -> Self {
        Self {
            counter: flag,
            fib_iterative: flag,
            fib_recursive: flag,
            fib_tailrec: flag,
            primes: flag,
            matrix_multiply: flag,
            argon2: flag,
            bulk_ops: flag,
            coremark: flag,
        }
    }
}

impl Default for ExecuteTestFilter {
    fn default() -> Self {
        Self::set_to(true)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CompileTestFilter {
    pub bz2: bool,
    pub pulldown_cmark: bool,
    pub spidermonkey: bool,
    pub ffmpeg: bool,
    pub coremark_minimal: bool,
    pub argon2: bool,
}

impl CompileTestFilter {
    pub fn set_to(flag: bool) -> Self {
        Self {
            bz2: flag,
            pulldown_cmark: flag,
            spidermonkey: flag,
            ffmpeg: flag,
            coremark_minimal: flag,
            argon2: flag,
        }
    }
}

impl Default for CompileTestFilter {
    fn default() -> Self {
        Self::set_to(true)
    }
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
