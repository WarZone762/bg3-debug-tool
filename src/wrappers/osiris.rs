use std::{
    ffi::{CStr, CString},
    fmt::Display,
};

use anyhow::bail;

use crate::{
    game_definitions::{OsiArgumentDesc, OsiArgumentValue, OsiString, ValueType},
    globals::Globals,
    hooks::osiris,
};

#[macro_export]
macro_rules! osi_fn {
    ($ident:ident) => {
        $crate::wrappers::osiris::FunctionCall {
            ident: stringify!($ident).to_string(),
            args: vec![],
        }.call()
    };
    ($ident:ident, $($arg:expr),*) => {
        $crate::wrappers::osiris::FunctionCall {
            ident: stringify!($ident).to_string(),
            args: vec![$($crate::wrappers::osiris::Value::from($arg)),*],
        }.call()
    };
}

#[derive(Debug)]
pub(crate) struct FunctionCall {
    pub ident: String,
    pub args: Vec<Value>,
}

impl syn::parse::Parse for FunctionCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Ident>()?;
        let content;
        syn::parenthesized!(content in input);
        let args = content.parse_terminated(Value::parse, syn::Token![,])?.into_iter().collect();

        Ok(Self { ident: name.to_string(), args })
    }
}

impl FunctionCall {
    pub fn call(&self) -> anyhow::Result<Option<Value>> {
        Function::new(&self.ident, self.args.len())?(self.args.iter().cloned())
    }
}

#[derive(Debug)]
pub(crate) enum Function {
    Call(Call),
    Query(Query),
}

impl<T: IntoIterator<Item = impl Into<Value>>> FnOnce<(T,)> for Function {
    type Output = anyhow::Result<Option<Value>>;

    extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
        self.call_fn(args.0)
    }
}

impl<T: IntoIterator<Item = impl Into<Value>>> FnMut<(T,)> for Function {
    extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
        self.call_fn(args.0)
    }
}

impl<T: IntoIterator<Item = impl Into<Value>>> Fn<(T,)> for Function {
    extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
        self.call_fn(args.0)
    }
}

impl Function {
    pub fn new(name: impl AsRef<str>, n_args: usize) -> anyhow::Result<Self> {
        let name = name.as_ref();
        if let Ok(f) = Call::new(name, n_args) {
            return Ok(Self::Call(f));
        }
        Ok(Self::Query(Query::new(name, n_args)?))
    }

    pub fn call_fn(
        &self,
        args: impl IntoIterator<Item = impl Into<Value>>,
    ) -> anyhow::Result<Option<Value>> {
        match self {
            Function::Call(x) => x.call(args).map(|_| None),
            Function::Query(x) => x.call(args).map(Some),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Call {
    name: String,
    handle: u32,
    args: Vec<ValueType>,
}

impl Call {
    pub fn new(name: impl AsRef<str>, n_args: usize) -> anyhow::Result<Self> {
        let name = name.as_ref();
        let osi_name = OsiString::from_bytes(format!("{name}/{n_args}").as_bytes());
        let hash = function_name_hash(name.as_bytes()) + n_args as u32;

        let Some(f) = Globals::osiris_globals().functions.find(hash, &osi_name.str) else {
            bail!("unable to find call '{name}' with {n_args} arguments");
        };

        let mut args = Vec::with_capacity(n_args);
        for (i, arg) in f.signature.params.params.iter().enumerate() {
            if f.signature.out_param_list.is_out_param(i) {
                bail!("{name}: the function has out parameters, so this should be a query");
            }
            args.push(arg.r#type.into());
        }

        Ok(Self { name: name.into(), handle: f.handle(), args })
    }

    pub fn call(&self, args: impl IntoIterator<Item = impl Into<Value>>) -> anyhow::Result<()> {
        let args = args.into_iter().map(Into::into).collect::<Vec<Value>>();
        if self.args.len() != args.len() {
            bail!(
                "call {}: incorrect number of arguments supplied: expected {}, got {}",
                self.name,
                self.args.len(),
                args.len(),
            );
        }

        let mut new_args = Vec::with_capacity(self.args.len());
        for (i, (provided, expected)) in args.iter().zip(&self.args).enumerate() {
            if let Some(arg) = provided.to_ffi(*expected) {
                new_args.push(arg);
            } else {
                bail!(
                    "call {}: incorrect function parameter type {i}: expected {expected:?}, got \
                     {} ({})",
                    self.name,
                    provided.type_str(),
                    provided,
                );
            }
        }

        OsiArgumentDesc::from_values(new_args, |args| {
            if !osiris::Call(self.handle, args.into()) {
                bail!("call {} failed with args '{}'", self.name, args);
            }
            Ok(())
        })
    }
}

#[derive(Debug)]
pub(crate) struct Query {
    name: String,
    handle: u32,
    args: Vec<Arg>,
}

impl Query {
    pub fn new(name: impl AsRef<str>, n_args: usize) -> anyhow::Result<Self> {
        let n_args = n_args + 1;
        let name = name.as_ref();
        let osi_name = OsiString::from_bytes(format!("{name}/{n_args}").as_bytes());
        let hash = function_name_hash(name.as_bytes()) + n_args as u32;

        let Some(f) = Globals::osiris_globals().functions.find(hash, &osi_name.str) else {
            bail!("unable to find query '{name}' with {} arguments", n_args - 1);
        };

        let mut args = Vec::with_capacity(n_args);
        for (i, arg) in f.signature.params.params.iter().enumerate() {
            if f.signature.out_param_list.is_out_param(i) {
                args.push(Arg::Out(arg.r#type.into()))
            } else {
                args.push(Arg::In(arg.r#type.into()));
            }
        }
        if !args.iter().any(|x| matches!(x, Arg::Out(_))) {
            bail!("{name}: the function has no out parameters, so this should be a call");
        }

        Ok(Self { name: name.into(), handle: f.handle(), args })
    }

    pub fn call(&self, args: impl IntoIterator<Item = impl Into<Value>>) -> anyhow::Result<Value> {
        let mut args = args.into_iter().map(Into::into).collect::<Vec<Value>>();
        if self.args.len() - 1 != args.len() {
            bail!(
                "query {}: incorrect number of arguments supplied: expected {}, got {}",
                self.name,
                self.args.len() - 1,
                args.len(),
            );
        }

        // let mut ret = OsiArgumentValue::none();
        let ret_i = self.args.iter().position(|x| matches!(x, Arg::Out(_))).unwrap();
        // ret.type_id = self.args[ret_i].r#type();
        args.insert(ret_i, Value::None);

        let mut new_args = Vec::with_capacity(self.args.len());
        for (i, (provided, expected)) in args.iter().zip(&self.args).enumerate() {
            if let Some(arg) = provided.to_ffi(expected.r#type()) {
                new_args.push(arg);
            } else {
                bail!(
                    "query {}: incorrect function parameter type {i}: expected {:?}, got {} ({})",
                    self.name,
                    expected.r#type(),
                    provided.type_str(),
                    provided,
                );
            }
        }

        OsiArgumentDesc::from_values(new_args, |args| {
            if !osiris::Query(self.handle, args.into()) {
                bail!("query {} failed with args {}", self.name, args);
            }
            Ok(Value::from_ffi(&args.iter().nth(ret_i).unwrap()))
        })
    }
}

#[derive(Debug)]
pub(crate) enum Arg {
    In(ValueType),
    Out(ValueType),
}

impl Arg {
    pub fn r#type(&self) -> ValueType {
        match self {
            Arg::In(x) => *x,
            Arg::Out(x) => *x,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Value {
    None,
    Int(i64),
    Float(f32),
    String(std::ffi::CString),
}

impl syn::parse::Parse for Value {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(if let Ok(lit) = input.parse::<syn::Lit>() {
            match lit {
                syn::Lit::Int(int) => Self::Int(int.base10_parse()?),
                syn::Lit::Float(float) => Self::Float(float.base10_parse()?),
                syn::Lit::Str(str) => Self::String(CString::new(str.value()).unwrap()),
                x => return Err(input.error(format!("unexpected literal '{x:?}'"))),
            }
        } else {
            let ident = input.parse::<syn::Ident>()?;

            match ident.to_string().as_str() {
                "None" => Self::None,
                x => return Err(input.error(format!("unknown Osiris argument type '{x}'"))),
            }
        })
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Int(value as _)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(CString::new(value).unwrap())
    }
}

impl From<&CStr> for Value {
    fn from(value: &CStr) -> Self {
        Self::String(value.into())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::None => f.write_str("None"),
            Value::Int(x) => Display::fmt(x, f),
            Value::Float(x) => Display::fmt(x, f),
            Value::String(x) => Display::fmt(x.to_str().unwrap(), f),
        }
    }
}

impl Value {
    pub fn to_ffi(&self, r#type: ValueType) -> Option<OsiArgumentValue> {
        match r#type {
            ValueType::None => return Some(OsiArgumentValue::none()),
            ValueType::Undefined => return Some(OsiArgumentValue::undefined()),
            _ => (),
        }

        Some(match self {
            Value::None => OsiArgumentValue::null(r#type),
            Value::Int(x) => match r#type {
                ValueType::Integer => OsiArgumentValue::int32(*x as _),
                ValueType::Integer64 => OsiArgumentValue::int64(*x),
                ValueType::Real => OsiArgumentValue::real(*x as _),
                _ => return None,
            },
            Value::Float(x) => match r#type {
                ValueType::Integer => OsiArgumentValue::int32(*x as _),
                ValueType::Integer64 => OsiArgumentValue::int64(*x as _),
                ValueType::Real => OsiArgumentValue::real(*x),
                _ => return None,
            },
            Value::String(x) => match r#type {
                ValueType::String => OsiArgumentValue::string(x.as_ptr()),
                ValueType::GuidString => OsiArgumentValue::guid_string(x.as_ptr()),
                ValueType::CharacterGuid => OsiArgumentValue::character_guid(x.as_ptr()),
                ValueType::ItemGuid => OsiArgumentValue::item_guid(x.as_ptr()),
                ValueType::Unknown21 => OsiArgumentValue::unknown21(x.as_ptr()),
                _ => return None,
            },
        })
    }

    pub fn from_ffi(value: &OsiArgumentValue) -> Self {
        unsafe {
            match value.type_id {
                ValueType::None => Self::None,
                ValueType::Integer => Self::Int(value.value.int32 as _),
                ValueType::Integer64 => Self::Int(value.value.int64),
                ValueType::Real => Self::Float(value.value.float),
                ValueType::String
                | ValueType::GuidString
                | ValueType::CharacterGuid
                | ValueType::ItemGuid
                | ValueType::Unknown21 => {
                    Self::String(std::ffi::CStr::from_ptr(value.value.string).into())
                }
                ValueType::Undefined => Self::None,
            }
        }
    }

    pub fn type_str(&self) -> &str {
        match self {
            Value::None => "None",
            Value::Int(_) => "Integer",
            Value::Float(_) => "Real",
            Value::String(_) => "String",
        }
    }
}

fn function_name_hash(str: &[u8]) -> u32 {
    let mut hash = 0u32;
    for char in str {
        if *char == b'\0' {
            break;
        }
        hash = (*char as u32 | 0x20) + 129 * (hash % 4294967);
    }

    hash
}
