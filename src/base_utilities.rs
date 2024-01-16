use std::{
    ffi::CStr,
    ops::{FromResidual, Try},
};

use libc::c_char;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Guid([u8; 16]);

impl From<uuid::Uuid> for Guid {
    fn from(value: uuid::Uuid) -> Self {
        let uuid = *value.as_bytes();
        let mut guid = [0u8; 16];

        // weird byte swapped format
        guid[0] = uuid[3];
        guid[1] = uuid[2];
        guid[2] = uuid[1];
        guid[3] = uuid[0];
        guid[4] = uuid[5];
        guid[5] = uuid[4];
        guid[6] = uuid[7];
        guid[7] = uuid[6];
        guid[8] = uuid[9];
        guid[9] = uuid[8];
        guid[10] = uuid[11];
        guid[11] = uuid[10];
        guid[12] = uuid[13];
        guid[13] = uuid[12];
        guid[14] = uuid[15];
        guid[15] = uuid[14];

        Self(guid)
    }
}

#[repr(C)]
pub struct CppOption<T> {
    flag: bool,
    val: T,
}

impl<T: Default> From<Option<T>> for CppOption<T> {
    fn from(value: Option<T>) -> Self {
        if let Some(val) = value {
            Self::from_output(val)
        } else {
            Self::from_residual(None)
        }
    }
}

impl<T> Try for CppOption<T> {
    type Output = T;
    type Residual = CppOption<T>;

    fn from_output(output: Self::Output) -> Self {
        CppOption {
            flag: true,
            val: output,
        }
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self.flag {
            true => std::ops::ControlFlow::Continue(self.val),
            false => std::ops::ControlFlow::Break(self),
        }
    }
}

impl<T> FromResidual for CppOption<T> {
    fn from_residual(residual: <Self as std::ops::Try>::Residual) -> Self {
        assert!(!residual.flag);
        residual
    }
}

impl<T: Default> FromResidual<Option<std::convert::Infallible>> for CppOption<T> {
    fn from_residual(_residual: Option<std::convert::Infallible>) -> Self {
        CppOption {
            flag: false,
            val: T::default(),
        }
    }
}

#[no_mangle]
unsafe extern "C" fn GuidParse(string: *const c_char) -> CppOption<Guid> {
    uuid::Uuid::parse_str(CStr::from_ptr(string).to_str().ok()?)
        .map(|x| x.into())
        .ok()
        .into()
}
