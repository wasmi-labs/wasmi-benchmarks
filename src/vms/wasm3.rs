use super::{elapsed_ms, BenchRuntime, BenchVm};
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
                Self::link_stubs(&mut module, imports).unwrap();
                module.compile().unwrap();
            }
        }
    }

    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let runtime = self.setup_runtime();
        let mut module = runtime.parse_and_load_module(wasm).unwrap();
        if matches!(self.compilation_mode, CompilationMode::Eager) {
            module.compile().unwrap();
        }
        Box::new(Wasm3Runtime { runtime })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        let runtime = self.setup_runtime();
        let mut module = runtime.parse_and_load_module(wasm).unwrap();
        module
            .link_closure::<(), u32, _>("env", "clock_ms", |_ctx, _args| Ok(elapsed_ms()))
            .unwrap();
        if matches!(self.compilation_mode, CompilationMode::Eager) {
            module.compile().unwrap();
        }
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
        wasm3::Runtime::new(&env, 2048).unwrap()
    }

    fn link_stubs(
        module: &mut wasm3::Module,
        imports: ModuleImportsIter,
    ) -> Result<(), wasm3::error::Error> {
        // Note: unfortunately the Wasm3 interpreter requires upfront bindings of all imported
        // functions but does not provide a way to query over Wasm module imports, thus
        // we need to provide this information via the benchmarking tool for Wasm3 to use.
        for import in imports {
            let module_name = import.module();
            let field_name = import.name();
            let func_type = match import.ty() {
                wasmi_new::ExternType::Global(ty) => {
                    unimplemented!("cannot stub link imported global variables but found: {ty:?}")
                }
                wasmi_new::ExternType::Table(ty) => {
                    unimplemented!("cannot stub link imported tables but found: {ty:?}")
                }
                wasmi_new::ExternType::Memory(ty) => {
                    unimplemented!("cannot stub link imported linear memories but found: {ty:?}")
                }
                wasmi_new::ExternType::Func(ty) => ty,
            };
            use ValType as Ty;
            // Note: unfortunately the Rust Wasm3 bindings do not allow to bind functions
            // with a dynamic (at runtime) type information, thus we need this ugly match.
            match (func_type.params(), func_type.results()) {
                // Without return type:
                ([], []) => Self::link_stub::<(), ()>(module, module_name, field_name)?,
                ([Ty::I32], []) => Self::link_stub::<i32, ()>(module, module_name, field_name)?,
                ([Ty::I32, Ty::I32], []) => {
                    Self::link_stub::<(i32, i32), ()>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32, Ty::I32], []) => {
                    Self::link_stub::<(i32, i32, i32), ()>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32], []) => {
                    Self::link_stub::<(i32, i32, i32, i32), ()>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32], []) => {
                    Self::link_stub::<(i32, i32, i32, i32, i32), ()>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32], []) => {
                    Self::link_stub::<(i32, i32, i32, i32, i32, i32), ()>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32], []) => {
                    Self::link_stub::<(i32, i32, i32, i32, i32, i32, i32), ()>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32], []) => {
                    Self::link_stub::<(i32, i32, i32, i32, i32, i32, i32, i32), ()>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                // With return type:
                ([], [Ty::I32]) => Self::link_stub::<(), i32>(module, module_name, field_name)?,
                ([Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<i32, i32>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i32), i32>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i32, i32), i32>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i32, i32, i32), i32>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i32, i32, i32, i32), i32>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i32, i32, i32, i32, i32), i32>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i32, i32, i32, i32, i32, i32), i32>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                (
                    [Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32],
                    [Ty::I32],
                ) => Self::link_stub::<(i32, i32, i32, i32, i32, i32, i32, i32), i32>(
                    module,
                    module_name,
                    field_name,
                )?,
                // Custom selected signatures:
                ([Ty::I32, Ty::I64, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i64, i32), i32>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I64, Ty::I32, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i64, i32, i32), i32>(module, module_name, field_name)?
                }
                ([Ty::I32, Ty::I32, Ty::I32, Ty::I64, Ty::I32], [Ty::I32]) => {
                    Self::link_stub::<(i32, i32, i32, i64, i32), i32>(
                        module,
                        module_name,
                        field_name,
                    )?
                }
                (
                    [Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I32, Ty::I64, Ty::I64, Ty::I32, Ty::I32],
                    [Ty::I32],
                ) => Self::link_stub::<(i32, i32, i32, i32, i32, i64, i64, i32, i32), i32>(
                    module,
                    module_name,
                    field_name,
                )?,
                _ => unimplemented!("found imported function with unsupported type: {func_type:?}"),
            }
        }
        Ok(())
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
