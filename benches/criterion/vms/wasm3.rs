use crate::utils::TestFilter;

use super::{BenchRuntime, BenchVm};

pub struct Wasm3;

struct Wasm3Runtime {
    runtime: wasm3::Runtime,
}

impl BenchVm for Wasm3 {
    fn name(&self) -> &'static str {
        "wasm3"
    }

    fn test_filter(&self) -> TestFilter {
        TestFilter {
            fib_tailrec: false,
            ..Default::default()
        }
    }

    fn compile(&self, wasm: &[u8]) {
        let env = wasm3::Environment::new().unwrap();
        env.parse_module(wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let env = wasm3::Environment::new().unwrap();
        let runtime = wasm3::Runtime::new(&env, 100_000).unwrap();
        runtime.parse_and_load_module(wasm).unwrap();
        Box::new(Wasm3Runtime { runtime })
    }
}

impl BenchRuntime for Wasm3Runtime {
    fn call(&mut self, input: i64) {
        let func = self.runtime.find_function::<i64, i64>("run").unwrap();
        func.call(input).unwrap();
    }
}
