/// Converts the `.wat` encoded `bytes` into `.wasm` encoded bytes.
pub fn wat2wasm(bytes: &[u8]) -> Vec<u8> {
    wat::parse_bytes(bytes).unwrap().into_owned()
}

#[derive(Debug, Copy, Clone)]
pub struct TestFilter {
    pub counter: bool,
    pub fib_iterative: bool,
    pub fib_recursive: bool,
    pub fib_tailrec: bool,
    pub primes: bool,
    pub matrix_multiply: bool,
    pub compile_bz2: bool,
    pub compile_pulldown_cmark: bool,
    pub compile_spidermonkey: bool,
    pub compile_ffmpeg: bool,
}

impl TestFilter {
    pub fn set_to(flag: bool) -> Self {
        Self {
            counter: flag,
            fib_iterative: flag,
            fib_recursive: flag,
            fib_tailrec: flag,
            primes: flag,
            matrix_multiply: flag,
            compile_bz2: flag,
            compile_pulldown_cmark: flag,
            compile_spidermonkey: flag,
            compile_ffmpeg: flag,
        }
    }
}

impl Default for TestFilter {
    fn default() -> Self {
        Self::set_to(true)
    }
}
