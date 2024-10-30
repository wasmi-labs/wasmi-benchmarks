use fluentbase_runtime::instruction::runtime_register_handlers;
use rwasm::{Config, Engine, Extern, Linker, Memory, MemoryType, Module, StackLimits, Store, Value};
use rwasm::engine::{RwasmConfig, StateRouterConfig, Tracer};
use rwasm::module::FuncIdx;
use rwasm::rwasm::instruction::InstructionExtra;
use rwasm::rwasm::{BinaryFormatWriter, RwasmModule};
use fluentbase_runtime::{Runtime, RuntimeContext};
use fluentbase_types::SharedContextInputV1;
use rwasm::engine::bytecode::Instruction;
use wasmi_new::ModuleImportsIter;
use super::{elapsed_ms, BenchRuntime, BenchVm};


pub struct Rwasm;

struct RwasmRuntime {
    store: Store<RuntimeContext>,
    engine: Engine,

    _instance: rwasm::Instance,
    func: rwasm::Func,
}

impl BenchVm for Rwasm {
    fn name(&self) -> &'static str {
      "rwasm"
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

        let mut config = Config::default();
        config
            .wasm_mutable_global(true)
            .wasm_saturating_float_to_int(true)
            .wasm_sign_extension(true)
            .wasm_multi_value(true)
            .wasm_bulk_memory(true)
            .wasm_reference_types(true)
            .wasm_tail_call(true)
            .wasm_extended_const(true)
            .set_cached_stacks(1024)
            .set_stack_limits(StackLimits::new(512, 11000, 100).unwrap());


        let import_linker = Runtime::new_import_linker();
        config.rwasm_config(RwasmConfig {
            state_router: None,
            entrypoint_name: None,
            import_linker: Some(import_linker),
            wrap_import_functions: false,
        });

        let ctx = RuntimeContext::new(wasm.to_vec());

        let engine = Engine::new(&config);

        let mut linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ctx);//.with_tracer(Tracer::default());

        runtime_register_handlers(&mut linker, &mut store);

        Memory::new(&mut store, MemoryType::new(1, Some(2)).unwrap()).unwrap();

        let original_module = Module::new(&engine, &wasm[..]).unwrap();

        let rwasm_module = RwasmModule::compile_with_config(wasm, &config).unwrap();

        let mut module_builder = rwasm_module.to_module_builder(&engine);

        for (k, v) in original_module.exports.iter() {
            if let Some(func_index) = v.into_func_idx() {
                let func_index = func_index.into_u32();
                if func_index < original_module.imports.len_funcs as u32 {
                    unreachable!("this is imported and exported func at the same time... ?")
                }
                let func_type = original_module.funcs[func_index as usize];
                let func_type = engine.resolve_func_type(&func_type, |v| v.clone());
                // remap exported a func type
                let new_func_type = engine.alloc_func_type(func_type);
                module_builder.funcs[func_index as usize] = new_func_type;
            }

            module_builder.push_export(k.clone(), *v);
        }

        let mut module = module_builder.finish();


        let entrypoint_func_index = module.funcs.len() - 1;

        module.start = Some(FuncIdx::from(entrypoint_func_index as u32));

        let instance_pre = linker.instantiate(&mut store, &module).unwrap();

        let instance = instance_pre.start(&mut store).unwrap();

        let func = instance.get_export(&store, "run")
            .and_then(Extern::into_func).unwrap();
        Box::new(RwasmRuntime {
            engine: store.engine().clone(),
            store,
            _instance: instance,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        todo!()
    }
}

impl BenchRuntime for RwasmRuntime {
    fn call(&mut self, input: i64) {

        let mut result = vec![];
        result.resize(1, Value::I64(0));
        self.func.call(&mut self.store, &[Value::I64(input)], &mut result).unwrap();
    }
}
