use crate::{FuncType, Val};
use std::collections::BTreeMap;

/// A runtime-neutral record of host function definitions.
///
/// Adapters whose native host functions are bound to a `Store` (and thus cannot keep a reusable
/// native linker across instantiations) record their host functions here in
/// [`RuntimeInstance::link_func`](crate::RuntimeInstance::link_func) and replay them into a fresh
/// native store/linker inside [`RuntimeInstance::instantiate`](crate::RuntimeInstance::instantiate).
///
/// Keyed by `(module, name)` just like a native linker resolves imports, so the key enforces
/// uniqueness. The recorded host function is a plain `fn` pointer and [`FuncType`] is `Clone`, so
/// nothing is store-bound and the record can be replayed any number of times.
#[derive(Default, Clone)]
pub struct Linker {
    funcs: BTreeMap<ImportName, (FuncType, HostFunc)>,
}

impl Linker {
    /// Creates a new empty [`Linker`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the host `func` with signature `ty` under `module::name`.
    pub fn define(&mut self, module: &str, name: &str, ty: FuncType, func: HostFunc) {
        self.funcs.insert(ImportName::new(module, name), (ty, func));
    }

    /// Yields `(module, name, &FuncType, func)` for each recorded host function.
    pub fn funcs(&self) -> impl Iterator<Item = (&str, &str, &FuncType, HostFunc)> {
        self.funcs
            .iter()
            .map(|(import_name, (ty, func))| (import_name.module(), import_name.name(), ty, *func))
    }
}

/// A host function usable through the runtime-neutral benchmark interface.
///
/// A plain (non-capturing) `fn` pointer so it is `Copy` and can be recorded in a [`Linker`] and
/// replayed into any runtime's native linker any number of times.
pub type HostFunc = fn(params: &[Val], results: &mut [Val]);

/// A Wasm import name.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ImportName {
    module: Box<str>,
    name: Box<str>,
}

impl ImportName {
    /// Creates a new [`ImportName`] from its parts.
    pub fn new(module: &str, name: &str) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
        }
    }

    /// Returns the `module` name part of the import name.
    pub fn module(&self) -> &str {
        self.module.as_ref()
    }

    /// Returns the `name` name part of the import name.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
