#![crate_type = "dylib"]

use benchmark_utils as utils;
use benchmark_utils::{
    ExecuteTestId, ModuleInstance, Runtime, RuntimeInstance, StartupTestId, TestId,
};
use makepad_stitch::{Engine, ExternVal, Func, Instance, Linker, Module, Store, Val, ValType};

pub struct Stitch;

struct StitchInstance {
    linker: utils::Linker,
}

struct StitchModule {
    store: Store,
    instance: Instance,
    params: Vec<Val>,
    results: Vec<Val>,
}

impl Runtime for Stitch {
    fn id(&self) -> &'static str {
        "stitch"
    }

    fn setup(&self, id: TestId) -> Option<Box<dyn RuntimeInstance>> {
        if !self.can_run(id) {
            return None;
        }
        Some(Box::new(StitchInstance {
            linker: utils::Linker::new(),
        }))
    }
}

impl Stitch {
    fn can_run(&self, id: TestId) -> bool {
        match id {
            TestId::Startup(id) => !matches!(id, StartupTestId::Ffmpeg),
            TestId::Execute(id) => !matches!(
                id,
                ExecuteTestId::FibonacciTail | ExecuteTestId::Argon2 | ExecuteTestId::Sort
            ),
        }
    }
}

impl RuntimeInstance for StitchInstance {
    fn link_func(
        &mut self,
        module: &str,
        name: &str,
        ty: utils::FuncType,
        func: fn(params: &[utils::Val], results: &mut [utils::Val]),
    ) {
        self.linker.define(module, name, ty, func);
    }

    fn instantiate(&self, wasm: &[u8]) -> Box<dyn ModuleInstance> {
        let mut store = Store::new(Engine::new());
        let mut linker = Linker::new();
        for (module, name, ty, func) in self.linker.funcs() {
            // Stitch only exposes the typed `Func::wrap` constructor (no untyped/dynamic host
            // function API), so `wrap_host_func` matches the runtime-neutral signature against the
            // concrete Rust closure types it needs. Host functions are bound to a `Store`, so they
            // are (re)built here against the fresh store on every instantiation.
            let host = wrap_host_func(&mut store, ty, func);
            linker.define(module, name, ExternVal::Func(host));
        }
        let module = Module::new(store.engine(), wasm).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        Box::new(StitchModule {
            store,
            instance,
            params: Vec::new(),
            results: Vec::new(),
        })
    }
}

impl ModuleInstance for StitchModule {
    fn call(
        &mut self,
        name: &str,
        params: &[utils::Val],
        results: &mut [utils::Val],
    ) -> anyhow::Result<()> {
        let Some(func) = self.instance.exported_func(name) else {
            anyhow::bail!("failed to find function")
        };
        assert_eq!(params.len(), func.type_(&self.store).params().len());
        assert_eq!(results.len(), func.type_(&self.store).results().len());
        self.prepare_params(params);
        self.prepare_results(&func);
        func.call(&mut self.store, &self.params[..], &mut self.results[..])?;
        self.write_back_results(results);
        Ok(())
    }
}

impl StitchModule {
    fn prepare_params(&mut self, params: &[utils::Val]) {
        self.params.clear();
        self.params
            .extend(params.iter().copied().map(from_utils_val));
    }

    fn prepare_results(&mut self, func: &Func) {
        self.results.clear();
        for ty in func.type_(&self.store).results() {
            self.results.push(default_val(*ty))
        }
    }

    fn write_back_results(&self, results: &mut [utils::Val]) {
        assert_eq!(results.len(), self.results.len());
        for (i, result) in results.iter_mut().enumerate() {
            *result = into_utils_val(self.results[i]);
        }
    }
}

/// Invokes a recorded host `func` that produces no results.
fn call(func: utils::HostFunc, params: &[utils::Val]) {
    func(params, &mut []);
}

/// Invokes a recorded host `func` that produces a single `i32` result.
fn ret_i32(func: utils::HostFunc, params: &[utils::Val]) -> i32 {
    let mut out = [utils::Val::I32(0)];
    func(params, &mut out);
    out[0].unwrap_i32()
}

/// Builds a stitch [`Func`] from a runtime-neutral signature and host `func` pointer.
///
/// Stitch only offers the typed `Func::wrap` constructor, so every signature imported by the
/// benchmark inputs is enumerated here. Any unsupported signature hits the catch-all `unimplemented`.
fn wrap_host_func(store: &mut Store, ty: &utils::FuncType, func: utils::HostFunc) -> Func {
    use utils::Val::{I32 as V32, I64 as V64};
    use utils::ValType::{I32, I64};
    match (ty.params(), ty.results()) {
        ([], []) => Func::wrap(store, move || call(func, &[])),
        ([], [I32]) => Func::wrap(store, move || -> i32 { ret_i32(func, &[]) }),
        ([I32], []) => Func::wrap(store, move |a: i32| call(func, &[V32(a)])),
        ([I32], [I32]) => Func::wrap(store, move |a: i32| -> i32 { ret_i32(func, &[V32(a)]) }),
        ([I32, I32], []) => Func::wrap(store, move |a: i32, b: i32| call(func, &[V32(a), V32(b)])),
        ([I32, I32], [I32]) => Func::wrap(store, move |a: i32, b: i32| -> i32 {
            ret_i32(func, &[V32(a), V32(b)])
        }),
        ([I32, I32, I32], []) => Func::wrap(store, move |a: i32, b: i32, c: i32| {
            call(func, &[V32(a), V32(b), V32(c)])
        }),
        ([I32, I32, I32], [I32]) => Func::wrap(store, move |a: i32, b: i32, c: i32| -> i32 {
            ret_i32(func, &[V32(a), V32(b), V32(c)])
        }),
        ([I32, I32, I32, I32], []) => Func::wrap(store, move |a: i32, b: i32, c: i32, d: i32| {
            call(func, &[V32(a), V32(b), V32(c), V32(d)])
        }),
        ([I32, I32, I32, I32], [I32]) => {
            Func::wrap(store, move |a: i32, b: i32, c: i32, d: i32| -> i32 {
                ret_i32(func, &[V32(a), V32(b), V32(c), V32(d)])
            })
        }
        ([I32, I32, I32, I32, I32], [I32]) => Func::wrap(
            store,
            move |a: i32, b: i32, c: i32, d: i32, e: i32| -> i32 {
                ret_i32(func, &[V32(a), V32(b), V32(c), V32(d), V32(e)])
            },
        ),
        ([I32, I32, I32, I32, I32, I32], [I32]) => Func::wrap(
            store,
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i32| -> i32 {
                ret_i32(func, &[V32(a), V32(b), V32(c), V32(d), V32(e), V32(f)])
            },
        ),
        ([I32, I64, I32], [I32]) => Func::wrap(store, move |a: i32, b: i64, c: i32| -> i32 {
            ret_i32(func, &[V32(a), V64(b), V32(c)])
        }),
        ([I32, I64, I32, I32], [I32]) => {
            Func::wrap(store, move |a: i32, b: i64, c: i32, d: i32| -> i32 {
                ret_i32(func, &[V32(a), V64(b), V32(c), V32(d)])
            })
        }
        ([I32, I32, I32, I64, I32], [I32]) => Func::wrap(
            store,
            move |a: i32, b: i32, c: i32, d: i64, e: i32| -> i32 {
                ret_i32(func, &[V32(a), V32(b), V32(c), V64(d), V32(e)])
            },
        ),
        ([I32, I32, I32, I32, I32, I64, I64, I32, I32], [I32]) => Func::wrap(
            store,
            move |a: i32, b: i32, c: i32, d: i32, e: i32, f: i64, g: i64, h: i32, i: i32| -> i32 {
                ret_i32(
                    func,
                    &[
                        V32(a),
                        V32(b),
                        V32(c),
                        V32(d),
                        V32(e),
                        V64(f),
                        V64(g),
                        V32(h),
                        V32(i),
                    ],
                )
            },
        ),
        _ => unimplemented!("the stitch adapter does not support host function signature {ty:?}"),
    }
}

fn default_val(ty: ValType) -> Val {
    match ty {
        ValType::I32 => Val::I32(0),
        ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0.0),
        ValType::F64 => Val::F64(0.0),
        _ => unreachable!(),
    }
}

fn from_utils_val(val: utils::Val) -> Val {
    match val {
        utils::Val::I32(val) => Val::I32(val),
        utils::Val::I64(val) => Val::I64(val),
        utils::Val::F32(val) => Val::F32(val),
        utils::Val::F64(val) => Val::F64(val),
    }
}

fn into_utils_val(val: Val) -> utils::Val {
    match val {
        Val::I32(val) => utils::Val::I32(val),
        Val::I64(val) => utils::Val::I64(val),
        Val::F32(val) => utils::Val::F32(val),
        Val::F64(val) => utils::Val::F64(val),
        _ => panic!(),
    }
}
