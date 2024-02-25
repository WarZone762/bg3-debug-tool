use std::ffi::CString;

use anyhow::bail;

use crate::{
    _println,
    game_definitions::{OsiArgumentDesc, OsiArgumentValue, OsiStringOwned, ValueType},
    globals::Globals,
    hooks, info, warn,
};

fn osi_get_arg_types(name: &str) -> Option<()> {
    for n_args in 0..7 {
        let osi_name = OsiStringOwned::from_bytes(format!("{name}/{n_args}").as_bytes());
        let hash = function_name_hash(name.as_bytes()) + n_args as u32;

        if let Some(osi_fn) = (**Globals::osiris_globals().functions).find(hash, &osi_name.string) {
            let mut arg_type = osi_fn.signatrue.params.params.head.next;

            _println!("{:?}", &mut *arg_type);
        }
    }

    Some(())
}

#[derive(Debug)]
pub(crate) struct OsiCall {
    pub ident: String,
    pub args: Vec<OsiArg>,
}

impl syn::parse::Parse for OsiCall {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Ident>()?;
        let content;
        syn::parenthesized!(content in input);
        let args = content.parse_terminated(OsiArg::parse, syn::Token![,])?.into_iter().collect();

        Ok(Self { ident: name.to_string(), args })
    }
}

impl OsiCall {
    pub fn call(&self) -> anyhow::Result<()> {
        let n_args = self.args.len();

        let osi_name = OsiStringOwned::from_bytes(format!("{}/{n_args}", self.ident).as_bytes());
        let hash = function_name_hash(self.ident.as_bytes()) + n_args as u32;

        let Some(osi_fn) = Globals::osiris_globals().functions.find(hash, &osi_name.string) else {
            bail!("unable to find call '{}' with {n_args} arguments", self.ident);
        };

        let osi_handle = osi_fn.handle();
        let osi_args = OsiArgumentDesc::from_values(self.args.iter().map(|x| x.to_ffi()));
        if !hooks::Call(osi_handle, osi_args) {
            bail!("call '{}' failed", self.ident)
        }

        Ok(())
    }

    pub fn query(&self) -> anyhow::Result<OsiArg> {
        let n_args = self.args.len() + 1;

        let osi_name = OsiStringOwned::from_bytes(format!("{}/{n_args}", self.ident).as_bytes());
        let hash = function_name_hash(self.ident.as_bytes()) + n_args as u32;

        let Some(osi_fn) = Globals::osiris_globals().functions.find(hash, &osi_name.string) else {
            bail!("unable to find query '{}' with {n_args} arguments", self.ident);
        };

        // let ret = osi_fn
        //     .signatrue
        //     .params
        //     .params
        //     .into_iter()
        //     .last()
        //     .map(|arg| OsiArgumentValue::null(arg.r#type))
        //     .unwrap_or_default();

        let mut in_args_count = 0;
        for (i, arg) in (&osi_fn.signatrue.params.params).into_iter().enumerate() {
            if osi_fn.signatrue.out_param_list.is_out_param(i) {
                info!("out param at {i}: {arg:?}");
            } else {
                let provided_type = self.args[in_args_count].to_ffi().type_id;
                if provided_type == arg.r#type {
                    info!("in param at {i}: {arg:?}");
                } else {
                    warn!(
                        "in param {i} wrong type: expected {:?}, got {provided_type:?}",
                        arg.r#type
                    );
                }
                in_args_count += 1;
            }
        }

        // let Some(ret) = osi_fn.signatrue.params.params.into_iter().last() else {
        //     bail!("query '{}' has no return parameters", self.ident)
        // };
        //
        // let ret = OsiArgumentValue::null(ret.r#type);

        let mut arg_type = osi_fn.signatrue.params.params.head.next;
        let mut ret = OsiArgumentValue::none();
        for _ in 0..n_args {
            ret.type_id = arg_type.item.r#type;
            arg_type = arg_type.next;
        }

        let osi_handle = osi_fn.handle();
        let osi_args = OsiArgumentDesc::from_values(
            self.args.iter().map(|x| x.to_ffi()).chain(std::iter::once(ret)),
        );
        if !hooks::Query(osi_handle, osi_args) {
            bail!("query '{}' failed", self.ident);
        }

        let mut out_arg = osi_args;
        for _ in 1..n_args {
            out_arg = out_arg.next_param
        }

        Ok(OsiArg::from_ffi(&out_arg.value))
    }
}

#[derive(Debug)]
pub(crate) enum OsiArg {
    None,
    I32(i32),
    I64(i64),
    F32(f32),
    String(std::ffi::CString),
    GuidString(std::ffi::CString),
    CharacterGuid(std::ffi::CString),
    ItemGuid(std::ffi::CString),
    Undefined,
}

impl syn::parse::Parse for OsiArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(if let Ok(lit) = input.parse::<syn::Lit>() {
            match lit {
                syn::Lit::Int(int) => match int.suffix() {
                    "" | "i32" => Self::I32(int.base10_parse()?),
                    "i64" => Self::I64(int.base10_parse()?),
                    "f32" => Self::F32(int.base10_parse()?),
                    x => return Err(input.error(format!("unsupported integer suffix '{x}'"))),
                },
                syn::Lit::Float(float) => match float.suffix() {
                    "" | "f32" => Self::F32(float.base10_parse()?),
                    x => return Err(input.error(format!("unsupported real suffix '{x}'"))),
                },
                syn::Lit::Str(str) => Self::String(CString::new(str.value()).unwrap()),
                _ => return Err(input.error("unexpected literal")),
            }
        } else {
            let ident = input.parse::<syn::Ident>()?;

            match ident.to_string().as_str() {
                "None" => Self::None,
                "Undefined" => Self::Undefined,
                x => {
                    let content;
                    syn::parenthesized!(content in input);
                    match x {
                        "GuidString" => Self::GuidString(
                            CString::new(content.parse::<syn::LitStr>()?.value()).unwrap(),
                        ),
                        "CharacterGuid" => Self::CharacterGuid(
                            CString::new(content.parse::<syn::LitStr>()?.value()).unwrap(),
                        ),
                        "ItemGuid" => Self::ItemGuid(
                            CString::new(content.parse::<syn::LitStr>()?.value()).unwrap(),
                        ),
                        x => return Err(input.error(format!("unknown Osiris argument type '{x}'"))),
                    }
                }
            }
        })
    }
}

impl OsiArg {
    pub fn to_ffi(&self) -> OsiArgumentValue {
        match self {
            OsiArg::None => OsiArgumentValue::none(),
            OsiArg::I32(i) => OsiArgumentValue::int32(*i),
            OsiArg::I64(i) => OsiArgumentValue::int64(*i),
            OsiArg::F32(r) => OsiArgumentValue::real(*r),
            OsiArg::String(s) => OsiArgumentValue::string(s.as_ptr()),
            OsiArg::GuidString(s) => OsiArgumentValue::guid_string(s.as_ptr()),
            OsiArg::CharacterGuid(s) => OsiArgumentValue::character_guid(s.as_ptr()),
            OsiArg::ItemGuid(s) => OsiArgumentValue::item_guid(s.as_ptr()),
            OsiArg::Undefined => OsiArgumentValue::undefined(),
        }
    }

    pub fn from_ffi(value: &OsiArgumentValue) -> Self {
        unsafe {
            match value.type_id {
                ValueType::None => Self::None,
                ValueType::Integer => Self::I32(value.value.int32),
                ValueType::Integer64 => Self::I64(value.value.int64),
                ValueType::Real => Self::F32(value.value.float),
                ValueType::String => {
                    Self::String(std::ffi::CStr::from_ptr(value.value.string).into())
                }
                ValueType::GuidString => {
                    Self::GuidString(std::ffi::CStr::from_ptr(value.value.string).into())
                }
                ValueType::CharacterGuid => {
                    Self::CharacterGuid(std::ffi::CStr::from_ptr(value.value.string).into())
                }
                ValueType::ItemGuid => {
                    Self::ItemGuid(std::ffi::CStr::from_ptr(value.value.string).into())
                }
                ValueType::Undefined => Self::Undefined,
            }
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
