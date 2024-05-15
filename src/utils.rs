use std::fmt;

/// Converts the `.wat` encoded `bytes` into `.wasm` encoded bytes.
pub fn wat2wasm(bytes: &[u8]) -> Vec<u8> {
    wat::parse_bytes(bytes).unwrap().into_owned()
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
    let wasm = match encoding {
        InputEncoding::Wasm => wasm_or_wat,
        InputEncoding::Wat => wat::parse_bytes(&wasm_or_wat[..])
            .unwrap_or_else(|error| panic!("failed to convert `.wat` to `.wasm`: {error}"))
            .to_vec(),
    };
    wasm
}
