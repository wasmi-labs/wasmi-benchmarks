use crate::{ModuleInstance, TypeMismatch, Val, ValType};

/// Extension to [`ModuleInstance`] to allow for simpler typed calls.
pub trait CallTyped {
    /// Call the function exported with `name` with `params` and return its `results`.
    ///
    /// # Note
    ///
    /// This is a concenience for the dynamically typed [`ModuleInstance::call`] API.
    fn call_typed<Params, Results>(
        &mut self,
        name: &str,
        params: Params,
    ) -> anyhow::Result<Results>
    where
        Params: WasmParams,
        Results: WasmResults;
}

impl<T> CallTyped for Box<T>
where
    T: ModuleInstance + ?Sized,
{
    fn call_typed<Params, Results>(&mut self, name: &str, params: Params) -> anyhow::Result<Results>
    where
        Params: WasmParams,
        Results: WasmResults,
    {
        let params = Params::params(params);
        let mut results = Results::results();
        self.call(name, params.as_ref(), results.as_mut()).unwrap();
        Ok(Results::from_results(results))
    }
}

/// Trait implemented by all primitive Wasm types.
pub trait WasmValue:
    Send + Sync + Copy + Clone + PartialEq + PartialOrd + Into<Val> + TryFrom<Val, Error = TypeMismatch>
{
    /// The statically known [`ValType`] of `Self`.
    const TY: ValType;
}

impl WasmValue for i32 {
    const TY: ValType = ValType::I32;
}
impl WasmValue for i64 {
    const TY: ValType = ValType::I64;
}
impl WasmValue for f32 {
    const TY: ValType = ValType::F32;
}
impl WasmValue for f64 {
    const TY: ValType = ValType::F64;
}

pub trait WasmParams {
    /// The parameters buffer, an array of [`Val`] with known length.
    type ParamBuffer: AsRef<[Val]>;

    /// Creates a new buffer for the parameters of the function call.
    fn params(self) -> Self::ParamBuffer;
}

impl WasmParams for () {
    type ParamBuffer = [Val; 0];

    #[inline]
    fn params(self) -> Self::ParamBuffer {
        []
    }
}

impl<T> WasmParams for T
where
    T: WasmValue,
{
    type ParamBuffer = [Val; 1];

    #[inline]
    fn params(self) -> Self::ParamBuffer {
        [self.into()]
    }
}

pub trait WasmResults {
    /// The results buffer, an array of [`Val`] with known length.
    type ResultsBuffer: AsMut<[Val]>;

    /// Creates a new buffer for the results of the function call.
    fn results() -> Self::ResultsBuffer;

    /// Creates `Self` from the updated results of the function call.
    fn from_results(results: Self::ResultsBuffer) -> Self;
}

impl WasmResults for () {
    type ResultsBuffer = [Val; 0];

    #[inline]
    fn results() -> Self::ResultsBuffer {
        []
    }

    #[inline]
    fn from_results(_results: Self::ResultsBuffer) -> Self {}
}

impl<T> WasmResults for T
where
    T: WasmValue,
{
    type ResultsBuffer = [Val; 1];

    #[inline]
    fn results() -> Self::ResultsBuffer {
        [Val::default_for_ty(T::TY)]
    }

    #[inline]
    fn from_results(results: Self::ResultsBuffer) -> Self {
        let [result] = results;
        <Self as TryFrom<Val>>::try_from(result).unwrap()
    }
}
