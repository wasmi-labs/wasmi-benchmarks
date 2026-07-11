#![crate_type = "dylib"]

use benchmark_utils::{BenchInstance, BenchRuntime, ExecuteTestFilter, TestFilter, elapsed_ms};

pub struct Wasm3;

struct Wasm3Runtime {
    runtime: wasm3::Runtime,
}

impl BenchRuntime for Wasm3 {
    fn name(&self) -> &'static str {
        "wasm3.lazy"
    }

    fn test_filter(&self) -> TestFilter {
        TestFilter {
            execute: ExecuteTestFilter {
                fib_tailrec: false,
                ..ExecuteTestFilter::set_to(true)
            },
            ..Default::default()
        }
    }

    fn compile(&self, wasm: &[u8]) {
        let env = wasm3::Environment::new().unwrap();
        env.parse_module(wasm).unwrap();
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchInstance> {
        let runtime = self.setup_runtime();
        runtime.parse_and_load_module(wasm).unwrap();
        Box::new(Wasm3Runtime { runtime })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let runtime = self.setup_runtime();
        let mut module = runtime.parse_and_load_module(wasm).unwrap();
        module
            .link_closure::<(), u32, _>("env", "clock_ms", |_ctx, _args| Ok(elapsed_ms()))
            .unwrap();
        module
            .find_function::<(), f32>("run")
            .unwrap()
            .call()
            .unwrap()
    }
}

impl Wasm3 {
    fn setup_runtime(&self) -> wasm3::Runtime {
        let env = wasm3::Environment::new().unwrap();
        wasm3::Runtime::new(&env, 8192).unwrap()
    }
}

impl BenchInstance for Wasm3Runtime {
    fn call(&mut self, input: i64) {
        let func = self.runtime.find_function::<i64, i64>("run").unwrap();
        func.call(input).unwrap();
    }
}
