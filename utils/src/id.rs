use core::fmt;

#[derive(Copy, Clone)]
pub enum TestId {
    Startup(StartupTestId),
    Execute(ExecuteTestId),
}

impl From<StartupTestId> for TestId {
    fn from(value: StartupTestId) -> Self {
        Self::Startup(value)
    }
}

impl From<ExecuteTestId> for TestId {
    fn from(value: ExecuteTestId) -> Self {
        Self::Execute(value)
    }
}

impl fmt::Display for TestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Startup(id) => id.fmt(f),
            Self::Execute(id) => id.fmt(f),
        }
    }
}

#[derive(Copy, Clone)]
pub enum StartupTestId {
    Erc20,
    Bz2,
    PulldownCmark,
    Spidermonkey,
    Ffmpeg,
    CoreMark,
    Argon2,
}

impl fmt::Display for StartupTestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Erc20 => "erc20",
            Self::Bz2 => "bz2",
            Self::PulldownCmark => "pulldown-cmark",
            Self::Spidermonkey => "spidermonkey",
            Self::Ffmpeg => "ffmpeg",
            Self::CoreMark => "coremark",
            Self::Argon2 => "argon2",
        };
        f.write_str(s)
    }
}

#[derive(Copy, Clone)]
pub enum ExecuteTestId {
    CounterLocal,
    CounterParam,
    CounterGlobal,
    FibonacciIter,
    FibonacciRec,
    FibonacciTail,
    Primes,
    MatrixMultiply,
    Argon2,
    BulkOps,
    CoreMark,
    Sort,
    PrimeSieve,
    Nbody,
    TinyKeccak,
    Mandelbrot,
    Spectralnorm,
    Compression,
}

impl fmt::Display for ExecuteTestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::CounterLocal => "counter-local",
            Self::CounterParam => "counter-param",
            Self::CounterGlobal => "counter-global",
            Self::FibonacciIter => "fibonacci-iter",
            Self::FibonacciRec => "fibonacci-rec",
            Self::FibonacciTail => "fibonacci-tail",
            Self::Primes => "primes",
            Self::MatrixMultiply => "matrix_mul",
            Self::Argon2 => "argon2",
            Self::BulkOps => "bulk-ops",
            Self::CoreMark => "coremark",
            Self::Sort => "sort",
            Self::PrimeSieve => "prime_sieve",
            Self::Nbody => "nbody",
            Self::TinyKeccak => "tiny_keccak",
            Self::Mandelbrot => "mandelbrot",
            Self::Spectralnorm => "spectralnorm",
            Self::Compression => "compression",
        };
        f.write_str(s)
    }
}
