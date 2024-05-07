use super::{BenchRuntime, BenchVm};
use crate::utils::{ExecuteTestFilter, TestFilter};
use wasmi_new::{core::ValType, ModuleImportsIter};

pub struct Wasm3 {
    pub compilation_mode: CompilationMode,
}

#[derive(Debug, Copy, Clone)]
pub enum CompilationMode {
    Lazy,
    Eager,
}

struct Wasm3Runtime {
    runtime: wasm3::Runtime,
}

impl BenchVm for Wasm3 {
    fn name(&self) -> &'static str {
        match self.compilation_mode {
            CompilationMode::Lazy => "wasm3.lazy",
            CompilationMode::Eager => "wasm3.eager",
        }
    }

    fn test_filter(&self) -> TestFilter {
        TestFilter {
            execute: ExecuteTestFilter {
                fib_tailrec: false,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn compile(&self, wasm: &[u8], imports: ModuleImportsIter) {
        let env = wasm3::Environment::new().unwrap();
        match self.compilation_mode {
            CompilationMode::Lazy => {
                env.parse_module(wasm).unwrap();
            }
            CompilationMode::Eager => {
                let runtime = wasm3::Runtime::new(&env, 1_000).unwrap();
                let mut module = runtime.parse_and_load_module(wasm).unwrap();
                // Self::link_wasi_stubs(&mut module).unwrap();
                // module
                //     .link_closure::<(), i32, _>(
                //         "env",
                //         "clock_ms",
                //         |_call_ctx, _args| -> Result<i32, wasm3::error::Trap> { todo!() },
                //     )
                //     .unwrap();
                module.link_wasi().unwrap();
                module.compile().unwrap();
            }
        }
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let env = wasm3::Environment::new().unwrap();
        let runtime = wasm3::Runtime::new(&env, 100_000).unwrap();
        let mut module = runtime.parse_and_load_module(wasm).unwrap();
        if matches!(self.compilation_mode, CompilationMode::Eager) {
            module.compile().unwrap();
        }
        Box::new(Wasm3Runtime { runtime })
    }
}

impl Wasm3 {
        Self::link_stub::<(), ()>(module, "bench", "start").unwrap();
        Self::link_stub::<(), ()>(module, "bench", "end").unwrap();

        Self::link_wasi_stub::<i32, ()>(module, "proc_exit").unwrap();

        Self::link_wasi_stub::<(i32, i32, i32, i32), i32>(module, "fd_read").unwrap();
        Self::link_wasi_stub::<(i32, i32), i32>(module, "fd_prestat_get").unwrap();
        Self::link_wasi_stub::<(i32, i32, i32), i32>(module, "fd_prestat_dir_name").unwrap();
        Self::link_wasi_stub::<(i32, i32), i32>(module, "fd_fdstat_get").unwrap();
        // Self::link_wasi_stub::<(i32, i32), i32>(module, "fd_fdstat_set_flags").unwrap();
        Self::link_wasi_stub::<(i32, i64, i32, i32), i32>(module, "fd_seek").unwrap();
        Self::link_wasi_stub::<(i32, i32, i32, i32), i32>(module, "fd_write").unwrap();
        Self::link_wasi_stub::<i32, i32>(module, "fd_close").unwrap();

        Self::link_wasi_stub::<(i32, i32, i32, i32, i32, i64, i64, i32, i32), i32>(
            module,
            "path_open",
        )
        .unwrap();
        Self::link_wasi_stub::<(i32, i32, i32, i32, i32), i32>(module, "path_filestat_get")
            .unwrap();
        // Self::link_wasi_stub::<(i32, i32, i32), i32>(module, "path_remove_directory").unwrap();
        // Self::link_wasi_stub::<(i32, i32, i32), i32>(module, "path_unlink_file").unwrap();

        Self::link_wasi_stub::<(i32, i32), i32>(module, "args_sizes_get").unwrap();
        Self::link_wasi_stub::<(i32, i32), i32>(module, "args_get").unwrap();

        // Self::link_wasi_stub::<(i32, i32), i32>(module, "environ_get").unwrap();
        // Self::link_wasi_stub::<(i32, i32), i32>(module, "environ_sizes_get").unwrap();

        // Self::link_wasi_stub::<(i32, i32), i32>(module, "clock_res_get").unwrap();
        // Self::link_wasi_stub::<(i32, i64, i32), i32>(module, "clock_time_get").unwrap();

        Ok(())
    }

    fn link_wasi_stub<Args, Ret>(
        module: &mut wasm3::Module,
    ) -> Result<(), wasm3::error::Error>
    where
        Args: wasm3::WasmArgs + 'static,
        Ret: wasm3::WasmType + 'static,
    {
        Self::link_stub::<Args, Ret>(module, "wasi_snapshot_preview1", function_name)
    }

    fn link_stub<Args, Ret>(
        module: &mut wasm3::Module,
        module_name: &str,
        function_name: &str,
    ) -> Result<(), wasm3::error::Error>
    where
        Args: wasm3::WasmArgs + 'static,
        Ret: wasm3::WasmType + 'static,
    {
        //     _call_ctx: wasm3::CallContext,
        //     _args: Args,
        // ) -> Result<Ret, wasm3::error::Trap> {
        //     unimplemented!()
        // }

        // module.link_closure::<Args, Ret, _>(module_name, function_name, noop::<Args, Ret>)
        module.link_closure::<Args, Ret, _>(
            module_name,
            function_name,
            |_call_ctx: wasm3::CallContext, _args: Args| unimplemented!(),
        )
    }
}

impl BenchRuntime for Wasm3Runtime {
    fn call(&mut self, input: i64) {
        let func = self.runtime.find_function::<i64, i64>("run").unwrap();
        func.call(input).unwrap();
    }
}
