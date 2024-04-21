use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    mem,
};

use anyhow::anyhow;

use super::{map::GameHash, GamePtr};
use crate::globals::Globals;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub(crate) struct FixedString {
    pub index: u32,
}

impl FixedString {
    const NULL_INDEX: u32 = 0xFFFFFFFF;

    pub fn new() -> Self {
        Self { index: Self::NULL_INDEX }
    }

    pub fn get(&self) -> Option<LSStringView> {
        if self.is_null() {
            return None;
        }

        let getter = Globals::static_symbols().ls__FixedString__GetString?;
        let mut sv = LSStringView::new();
        getter(self.into(), GamePtr::new(&mut sv));
        Some(sv)
    }

    pub fn metadata(&self) -> Option<&FixedStringHeader> {
        if self.is_null() {
            return None;
        }

        let str = self.get()?.data;
        unsafe {
            mem::transmute(str.sub(mem::size_of::<FixedStringHeader>()) as *const FixedStringHeader)
        }
    }

    pub fn is_null(&self) -> bool {
        self.index == Self::NULL_INDEX
    }

    pub fn as_str(&self) -> &str {
        self.get().map(|x| x.as_str()).expect("failed to find FixedString")
    }
}

impl GameHash for FixedString {
    fn hash(&self) -> u64 {
        if !self.is_null() {
            self.metadata().unwrap().hash as _
        } else {
            0
        }
    }
}

impl TryInto<String> for FixedString {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        self.get().map(|x| x.into()).ok_or(anyhow!("failed to find FixedString"))
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FixedStringHeader {
    hash: u32,
    ref_count: u32,
    length: u32,
    id: u32,
    next_free_index: u64,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LSStringView<'a> {
    data: *const u8,
    size: i32,
    marker: PhantomData<&'a str>,
}

impl Debug for LSStringView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LSStringView")
            .field("data", &self.as_str())
            .field("size", &self.size)
            .finish()
    }
}

impl Display for LSStringView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Into<String> for LSStringView<'_> {
    fn into(self) -> String {
        self.to_string()
    }
}

impl<'a> LSStringView<'a> {
    pub fn new() -> Self {
        Self { data: std::ptr::null(), size: 0, marker: PhantomData::default() }
    }

    pub fn as_str(&self) -> &'a str {
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.data, self.size as _))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Guid(u64, u64);

impl GameHash for Guid {
    fn hash(&self) -> u64 {
        self.0 ^ self.1
    }
}