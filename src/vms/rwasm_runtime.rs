use fluentbase_runtime::instruction::runtime_register_handlers;
use rwasm::{Config, Engine, Extern, Linker, Memory, MemoryType, Module, StackLimits, Store, Value};
use rwasm::engine::{RwasmConfig, StateRouterConfig, Tracer};
use rwasm::module::FuncIdx;
use rwasm::rwasm::instruction::InstructionExtra;
use rwasm::rwasm::{BinaryFormat, BinaryFormatWriter, RwasmModule};
use fluentbase_runtime::{Runtime, RuntimeContext};
use fluentbase_types::SharedContextInputV1;
use rwasm::engine::bytecode::Instruction;
use wasmi_new::ModuleImportsIter;
use super::{elapsed_ms, BenchRuntime, BenchVm};
use fluentbase_codec::Encoder;

pub struct RwasmRm;

struct RwasmRuntime {
    runtime: fluentbase_runtime::Runtime
}

impl RwasmRm {
    #[inline(always)]
    fn wasm2rwasm(wasm_binary: &[u8]) -> Vec<u8> {
        let rwasm_module = RwasmRm::rwasm_module(wasm_binary);

        let length = rwasm_module.encoded_length();
        let mut rwasm_bytecode = vec![0u8; length];
        let mut binary_format_writer = BinaryFormatWriter::new(&mut rwasm_bytecode);
        rwasm_module
            .write_binary(&mut binary_format_writer)
            .expect("failed to encode rwasm bytecode");
        rwasm_bytecode
    }

    pub const STATE_DEPLOY: u32 = 1;

    pub const STATE_MAIN: u32 = 0;
    #[inline(always)]
    fn rwasm_module(wasm_binary: &[u8]) -> RwasmModule {
        let mut config = RwasmModule::default_config(None);
        config.rwasm_config(RwasmConfig {
            state_router: Some(StateRouterConfig {
                states: Box::new([
                    ("deploy".to_string(), RwasmRm::STATE_DEPLOY),
                    ("main".to_string(), RwasmRm::STATE_MAIN),
                ]),
                opcode: Instruction::Call(0x0002u32.into()),
            }),
            entrypoint_name: None,
            import_linker: Some(Runtime::new_import_linker()),
            wrap_import_functions: true,
        });
        RwasmModule::compile_with_config(wasm_binary, &config).unwrap()
    }
}

impl BenchVm for RwasmRm {
    fn name(&self) -> &'static str {
      "rwasm-rm"
    }

    fn compile(&self, wasm: &[u8], _imports: ModuleImportsIter) {
        let mut config = Config::default();

        config.rwasm_config(RwasmConfig {
            state_router: None,
            entrypoint_name: None,
            import_linker: None,
            wrap_import_functions: false,
        });

        let engine = Engine::new(&config);

        let rwasm_module = RwasmModule::compile_with_config(wasm, &config).unwrap();

        let mut module_builder = rwasm_module.to_module_builder(&engine);

        module_builder.finish();
    }



    fn load(&self, wasm: &[u8]) -> Box<dyn BenchRuntime> {
        let rwasm_binary = RwasmRm::wasm2rwasm(wasm);
        let context_input = SharedContextInputV1 {
            block: Default::default(),
            tx: Default::default(),
            contract: Default::default(),
        }
            .encode_to_vec(0);

        let mut ctx = RuntimeContext::new(rwasm_binary)

            .with_fuel_limit(100_000_000_000)
            .with_input(context_input)
            .with_state(RwasmRm::STATE_MAIN);

        let mut runtime = Runtime::new(ctx);

        runtime.data_mut().clear_output();
        Box::new(RwasmRuntime {
            runtime,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        todo!()
    }
}

impl BenchRuntime for RwasmRuntime {
    fn call(&mut self, input: i64) {

        self.runtime.call();
    }
}
