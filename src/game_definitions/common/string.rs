use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    mem,
    ops::Deref,
};

use super::{map::GameHash, GamePtr};
use crate::globals::Globals;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub(crate) struct FixedString {
    pub index: u32,
}

impl Default for FixedString {
    fn default() -> Self {
        Self { index: Self::NULL_INDEX }
    }
}

impl FixedString {
    const NULL_INDEX: u32 = 0xFFFFFFFF;

    pub fn get(&self) -> Option<LSStringView<'static>> {
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

impl Display for FixedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get() {
            Some(x) => f.write_str(x.as_str()),
            None => Ok(()),
        }
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

impl From<FixedString> for String {
    fn from(value: FixedString) -> Self {
        value.as_str().into()
    }
}

impl From<&FixedString> for String {
    fn from(value: &FixedString) -> Self {
        value.as_str().into()
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

impl From<LSStringView<'_>> for String {
    fn from(value: LSStringView<'_>) -> Self {
        value.to_string()
    }
}

impl<'a> Deref for LSStringView<'a> {
    type Target = str;

    fn deref(&self) -> &'a Self::Target {
        self.as_str()
    }
}

impl PartialEq for LSStringView<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}
impl Eq for LSStringView<'_> {}

impl PartialOrd for LSStringView<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LSStringView<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<'a> LSStringView<'a> {
    pub fn new() -> Self {
        Self { data: std::ptr::null(), size: 0, marker: PhantomData }
    }

    pub fn as_str(&self) -> &'a str {
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.data, self.size as _))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Guid(pub u64, pub u64);

impl GameHash for Guid {
    fn hash(&self) -> u64 {
        self.0 ^ self.1
    }
}
