#![crate_type = "dylib"]

use core::fmt;

/// A typed Wasm value.
#[derive(Debug, Copy, Clone)]
pub enum Val {
    /// A Wasm `i32` value.
    I32(i32),
    /// A Wasm `i64` value.
    I64(i64),
    /// A Wasm `f32` value.
    F32(f32),
    /// A Wasm `f64` value.
    F64(f64),
}

impl Val {
    /// Returns the [`ValType`] of `self`.
    #[inline]
    pub fn ty(self) -> ValType {
        match self {
            Self::I32(_) => ValType::I32,
            Self::I64(_) => ValType::I64,
            Self::F32(_) => ValType::F32,
            Self::F64(_) => ValType::F64,
        }
    }
}

/// A Wasm type.
#[derive(Debug, Copy, Clone)]
pub enum ValType {
    /// Wasm `i32` type.
    I32,
    /// Wasm `i64` type.
    I64,
    /// Wasm `f32` type.
    F32,
    /// Wasm `f64` type.
    F64,
}

impl fmt::Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ValType::I32 => "i32",
            ValType::I64 => "i64",
            ValType::F32 => "f32",
            ValType::F64 => "f64",
        };
        f.write_str(s)
    }
}

macro_rules! impl_val {
    ( $( $camel:ident($snake:ident) = { fn $unwrap:ident }),* $(,)? ) => {
        $(
            impl From<::core::primitive::$snake> for Val {
                #[inline]
                fn from(value: ::core::primitive::$snake) -> Self {
                    Self::$camel(value)
                }
            }
        )*

        impl Val {
            $(
                #[doc = concat!("Unwraps a value of type [`ValType::", stringify!($camel), "`] or panics.")]
                #[inline]
                pub fn $unwrap(self) -> ::core::primitive::$snake {
                    match self {
                        Self::$camel(val) => val,
                        found => {
                            let required = ValType::$camel;
                            let found = found.ty();
                            panic!("mismatched type: required {required} but found {found}")
                        }
                    }
                }

                #[doc = concat!("Returns a value of type [`ValType::", stringify!($camel), "`] or `None`.")]
                #[inline]
                pub fn $snake(self) -> Option<::core::primitive::$snake> {
                    match self {
                        Self::$camel(val) => Some(val),
                        _ => None,
                    }
                }
            )*
        }
    };
}
impl_val! {
    I32(i32) = { fn unwrap_i32 },
    I64(i64) = { fn unwrap_i64 },
    F32(f32) = { fn unwrap_f32 },
    F64(f64) = { fn unwrap_f64 },
}

/// A Wasm runtime that is capable of being benchmarked.
pub trait BenchRuntime {
    /// Returns the name of the Wasm runtime and its configuration.
    fn name(&self) -> &'static str;

    /// Returns `true` if `self` can run the test with the given `id`.
    fn can_run(&self, id: TestId) -> bool;

    /// Compiles the `wasm` using the Wasm runtime and its configuration.
    fn compile(&self, wasm: &[u8]);

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
    CoreMarkMinimal,
    Argon2,
}

#[derive(Copy, Clone)]
pub enum ExecuteTestId {
    Counter,
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
