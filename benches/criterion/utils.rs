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
}

impl CompileTestFilter {
    pub fn set_to(flag: bool) -> Self {
        Self {
            bz2: flag,
            pulldown_cmark: flag,
            spidermonkey: flag,
            ffmpeg: flag,
        }
    }
}

impl Default for CompileTestFilter {
    fn default() -> Self {
        Self::set_to(true)
    }
}
