use super::{BenchRuntime, BenchVm};
use rwasm::{
    engine::RwasmConfig,
    module::FuncIdx,
    rwasm::RwasmModule,
    Config,
    Engine,
    Extern,
    Linker,
    Module,
    StackLimits,
    Store,
    Value,
};
use wasmi_new::ModuleImportsIter;

pub struct Rwasm;

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

        config.rwasm_config(RwasmConfig {
            state_router: None,
            entrypoint_name: None,
            import_linker: None,
            wrap_import_functions: false,
        });

        let engine = Engine::new(&config);

        let mut linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());

        // Memory::new(&mut store, MemoryType::new(1, Some(2)).unwrap()).unwrap();

        let original_module = Module::new(&engine, &wasm[..]).unwrap();
        let rwasm_module = RwasmModule::from_module(&original_module);

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

        let func = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap()
            .get_export(&store, "run")
            .and_then(Extern::into_func)
            .unwrap();
        Box::new(RwasmRuntime {
            engine: store.engine().clone(),
            store,
            func,
        })
    }

    fn coremark(&self, wasm: &[u8]) -> f32 {
        todo!()
    }
}

struct RwasmRuntime {
    store: Store<()>,
    engine: Engine,
    func: rwasm::Func,
}

impl BenchRuntime for RwasmRuntime {
    fn call(&mut self, input: i64) {
        let mut result = vec![];
        result.resize(1, Value::I64(0));
        self.func
            .call(&mut self.store, &[Value::I64(input)], &mut result)
            .unwrap();
    }
}
