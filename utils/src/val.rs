use core::fmt;

/// A typed Wasm value.
#[derive(Debug, Copy, Clone)]
pub enum Val {
    /// A Wasm `i32` value.
    I32(i32),
    /// A Wasm `i64` value.
    I64(i64),
    /// A Wasm `f32` value.
    F32(f32),
    /// A Wasm `f64` value.
    F64(f64),
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Val::I32(val) => val.fmt(f),
            Val::I64(val) => val.fmt(f),
            Val::F32(val) => val.fmt(f),
            Val::F64(val) => val.fmt(f),
        }
    }
}

impl Val {
    /// Returns the [`ValType`] of `self`.
    #[inline]
    pub fn ty(self) -> ValType {
        match self {
            Self::I32(_) => ValType::I32,
            Self::I64(_) => ValType::I64,
            Self::F32(_) => ValType::F32,
            Self::F64(_) => ValType::F64,
        }
    }

    /// Returns the default [`Val`] for `ty`.
    pub fn default_for_ty(ty: ValType) -> Self {
        match ty {
            ValType::I32 => Self::I32(0),
            ValType::I64 => Self::I64(0),
            ValType::F32 => Self::F32(0.0),
            ValType::F64 => Self::F64(0.0),
        }
    }
}

/// The type signature of a Wasm function: its parameter and result types.
#[derive(Debug, Clone)]
pub struct FuncType {
    params: Box<[ValType]>,
    results: Box<[ValType]>,
}

impl FuncType {
    /// Creates a new [`FuncType`] from its `params` and `results` types.
    pub fn new(
        params: impl IntoIterator<Item = ValType>,
        results: impl IntoIterator<Item = ValType>,
    ) -> Self {
        Self {
            params: params.into_iter().collect(),
            results: results.into_iter().collect(),
        }
    }

    /// Returns the parameter types of the [`FuncType`].
    pub fn params(&self) -> &[ValType] {
        &self.params
    }

    /// Returns the result types of the [`FuncType`].
    pub fn results(&self) -> &[ValType] {
        &self.results
    }
}

/// A Wasm type.
#[derive(Debug, Copy, Clone)]
pub enum ValType {
    /// Wasm `i32` type.
    I32,
    /// Wasm `i64` type.
    I64,
    /// Wasm `f32` type.
    F32,
    /// Wasm `f64` type.
    F64,
}

impl fmt::Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ValType::I32 => "i32",
            ValType::I64 => "i64",
            ValType::F32 => "f32",
            ValType::F64 => "f64",
        };
        f.write_str(s)
    }
}

macro_rules! impl_val {
    ( $( $camel:ident($snake:ident) = { fn $unwrap:ident }),* $(,)? ) => {
        $(
            impl From<::core::primitive::$snake> for Val {
                #[inline]
                fn from(value: ::core::primitive::$snake) -> Self {
                    Self::$camel(value)
                }
            }
        )*

        impl Val {
            $(
                #[doc = concat!("Unwraps a value of type [`ValType::", stringify!($camel), "`] or panics.")]
                #[inline]
                pub fn $unwrap(self) -> ::core::primitive::$snake {
                    match self {
                        Self::$camel(val) => val,
                        found => {
                            let required = ValType::$camel;
                            let found = found.ty();
                            panic!("mismatched type: required {required} but found {found}")
                        }
                    }
                }

                #[doc = concat!("Returns a value of type [`ValType::", stringify!($camel), "`] or `None`.")]
                #[inline]
                pub fn $snake(self) -> Option<::core::primitive::$snake> {
                    match self {
                        Self::$camel(val) => Some(val),
                        _ => None,
                    }
                }
            )*
        }
    };
}
impl_val! {
    I32(i32) = { fn unwrap_i32 },
    I64(i64) = { fn unwrap_i64 },
    F32(f32) = { fn unwrap_f32 },
    F64(f64) = { fn unwrap_f64 },
}
