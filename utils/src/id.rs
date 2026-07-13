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
            TestId::Startup(id) => id.fmt(f),
            TestId::Execute(id) => id.fmt(f),
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
            StartupTestId::Erc20 => "erc20",
            StartupTestId::Bz2 => "bz2",
            StartupTestId::PulldownCmark => "pulldown-cmark",
            StartupTestId::Spidermonkey => "spidermonkey",
            StartupTestId::Ffmpeg => "ffmpeg",
            StartupTestId::CoreMark => "coremark",
            StartupTestId::Argon2 => "argon2",
        };
        f.write_str(s)
    }
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

impl fmt::Display for ExecuteTestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ExecuteTestId::CounterLocal => "counter-local",
            ExecuteTestId::CounterParam => "counter-param",
            ExecuteTestId::FibonacciIter => "fibonacci-rec",
            ExecuteTestId::FibonacciRec => "fibonacci-iter",
            ExecuteTestId::FibonacciTail => "fibonacci-tail",
            ExecuteTestId::Primes => "primes",
            ExecuteTestId::MatrixMultiply => "matmul",
            ExecuteTestId::Argon2 => "argon2",
            ExecuteTestId::BulkOps => "bulk-ops",
            ExecuteTestId::CoreMark => "coremark",
        };
        f.write_str(s)
    }
}
