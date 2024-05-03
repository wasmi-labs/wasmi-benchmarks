pub mod tinywasm;
pub mod wasmer;
pub mod wasmi_new;
pub mod wasmi_old;
pub mod wasmtime;

pub use self::tinywasm::Tinywasm;
pub use self::wasmer::Wasmer;
pub use self::wasmi_new::WasmiNew;
pub use self::wasmi_old::WasmiOld;
pub use self::wasmtime::Wasmtime;
use crate::utils::TestFilter;

pub trait BenchVm {
    fn name(&self) -> &'static str;
    fn test_filter(&self) -> TestFilter {
        TestFilter::default()
    }
    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime>;
}

pub trait BenchRuntime {
    fn call(&mut self, input: i64);
}
