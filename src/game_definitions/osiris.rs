use std::{
    alloc,
    ffi::CStr,
    fmt::{Debug, Display},
    marker,
};

use itertools::Itertools;

use super::{GamePtr, PtrOrBuf};
use crate::warn;

#[derive(Debug, Clone, Copy)]
pub(crate) struct OsirisStaticGlobals {
    pub variables: *const (),
    pub types: *const (),
    pub enums: *const (),
    pub functions: GamePtr<GamePtr<FunctionDb>>,
    pub objects: *const (),
    pub goals: *const (),
    pub adapters: *const (),
    pub databases: *const (),
    pub nodes: *const (),
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct OsiArgumentDesc<'a> {
    pub next_param: Option<&'a Self>,
    pub value: OsiArgumentValue,
}

impl<'a> OsiArgumentDesc<'a> {
    pub fn new(value: OsiArgumentValue) -> Self {
        Self { next_param: None, value }
    }

    pub fn prepend(&'a self, value: OsiArgumentValue) -> Self {
        Self { next_param: Some(self), value }
    }

    pub fn prepend_all<F, T>(&self, mut iter: impl Iterator<Item = OsiArgumentValue>, f: F) -> T
    where
        F: for<'b> FnOnce(&'b OsiArgumentDesc<'b>) -> T,
    {
        match iter.next() {
            None => f(self),
            Some(x) => self.prepend(x).prepend_all(iter, f),
        }
    }

    pub fn from_values<F, T>(
        iter: impl IntoIterator<
            Item = OsiArgumentValue,
            IntoIter = impl DoubleEndedIterator<Item = OsiArgumentValue>,
        >,
        f: F,
    ) -> T
    where
        F: for<'b> FnOnce(&'b OsiArgumentDesc<'b>) -> T,
    {
        let mut rev = iter.into_iter().rev();
        let list = Self::new(rev.next().unwrap_or(OsiArgumentValue::undefined()));

        list.prepend_all(rev, f)
    }

    pub fn iter(&self) -> OsiArgumentDescIter {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a OsiArgumentDesc<'a> {
    type IntoIter = OsiArgumentDescIter<'a>;
    type Item = OsiArgumentValue;

    fn into_iter(self) -> Self::IntoIter {
        OsiArgumentDescIter { current: Some(self) }
    }
}

impl Display for OsiArgumentDesc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.iter().join(", "))
    }
}

#[derive(Debug)]
pub(crate) struct OsiArgumentDescIter<'a> {
    current: Option<&'a OsiArgumentDesc<'a>>,
}

impl<'a> Iterator for OsiArgumentDescIter<'a> {
    type Item = OsiArgumentValue;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.current?.value;
        self.current = self.current?.next_param;
        Some(value)
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub(crate) struct OsiArgumentValue {
    pub value: OsiArgumentValueUnion,
    pub type_id: ValueType,
    unknown: bool,
}

impl Debug for OsiArgumentValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self.type_id {
            ValueType::None => "None".into(),
            ValueType::Integer => unsafe { format!("Integer({})", self.value.int32) },
            ValueType::Integer64 => unsafe { format!("Integer64({})", self.value.int64) },
            ValueType::Real => unsafe { format!("Real({})", self.value.float) },
            ValueType::String => unsafe {
                format!("String({})", CStr::from_ptr(self.value.string as _).to_str().unwrap())
            },
            ValueType::GuidString => unsafe {
                format!("GuidString({})", CStr::from_ptr(self.value.string as _).to_str().unwrap())
            },
            ValueType::CharacterGuid => unsafe {
                format!(
                    "CharacterGuid({})",
                    CStr::from_ptr(self.value.string as _).to_str().unwrap()
                )
            },
            ValueType::ItemGuid => unsafe {
                format!("ItemGuid({})", CStr::from_ptr(self.value.string as _).to_str().unwrap())
            },
            ValueType::Undefined => "Undefined".into(),
            x => unsafe {
                format!(
                    "Unknown{}({})",
                    x as u16,
                    CStr::from_ptr(self.value.string as _).to_str().unwrap()
                )
            },
        };

        f.debug_struct("OsiArgumentValue")
            .field("value", &value)
            .field("value_type", &self.type_id)
            .field("unknown", &self.unknown)
            .finish()
    }
}

impl Display for OsiArgumentValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.type_id {
            ValueType::None => f.write_str("None"),
            ValueType::Integer => unsafe { Display::fmt(&self.value.int32, f) },
            ValueType::Integer64 => unsafe { Display::fmt(&self.value.int64, f) },
            ValueType::Real => unsafe { Display::fmt(&self.value.float, f) },
            ValueType::Undefined => f.write_str("Undefined"),
            _ => unsafe {
                Display::fmt(CStr::from_ptr(self.value.string as _).to_str().unwrap(), f)
            },
        }
    }
}

impl OsiArgumentValue {
    pub fn null(type_id: ValueType) -> Self {
        Self { type_id, ..Default::default() }
    }

    pub fn none() -> Self {
        Self { type_id: ValueType::None, ..Default::default() }
    }

    pub fn int32(int32: i32) -> Self {
        Self { value: OsiArgumentValueUnion { int32 }, type_id: ValueType::Integer, unknown: false }
    }

    pub fn int64(int64: i64) -> Self {
        Self {
            value: OsiArgumentValueUnion { int64 },
            type_id: ValueType::Integer64,
            unknown: false,
        }
    }

    pub fn real(float: f32) -> Self {
        Self { value: OsiArgumentValueUnion { float }, type_id: ValueType::Real, unknown: false }
    }

    pub fn string(string: *const i8) -> Self {
        Self { value: OsiArgumentValueUnion { string }, type_id: ValueType::String, unknown: false }
    }

    pub fn guid_string(string: *const i8) -> Self {
        Self {
            value: OsiArgumentValueUnion { string },
            type_id: ValueType::GuidString,
            unknown: false,
        }
    }

    pub fn character_guid(string: *const i8) -> Self {
        Self {
            value: OsiArgumentValueUnion { string },
            type_id: ValueType::CharacterGuid,
            unknown: false,
        }
    }

    pub fn item_guid(string: *const i8) -> Self {
        Self {
            value: OsiArgumentValueUnion { string },
            type_id: ValueType::ItemGuid,
            unknown: false,
        }
    }

    pub fn unknown21(string: *const i8) -> Self {
        Self {
            value: OsiArgumentValueUnion { string },
            type_id: ValueType::Unknown21,
            unknown: false,
        }
    }

    pub fn undefined() -> Self {
        Self { type_id: ValueType::Undefined, ..Default::default() }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) union OsiArgumentValueUnion {
    pub string: *const i8,
    pub int32: i32,
    pub int64: i64,
    pub float: f32,
}

impl Default for OsiArgumentValueUnion {
    fn default() -> Self {
        Self { int64: 0 }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum ValueType {
    #[default]
    None = 0,
    Integer = 1,
    Integer64 = 2,
    Real = 3,
    String = 4,
    GuidString = 5,
    CharacterGuid = 6,
    ItemGuid = 7,
    Unknown8 = 8,
    Unknown9 = 9,
    Unknown10 = 10,
    Unknown11 = 11,
    Unknown12 = 12,
    Unknown13 = 13,
    Unknown14 = 14,
    Unknown15 = 15,
    Unknown16 = 16,
    Unknown17 = 17,
    Unknown18 = 18,
    Unknown19 = 19,
    Unknown20 = 20,
    Unknown21 = 21,
    Unknown22 = 22,
    Unknown23 = 23,
    Unknown24 = 24,
    Unknown25 = 25,
    Unknown26 = 26,
    Unknown27 = 27,
    Unknown28 = 28,
    Unknown29 = 29,
    Unknown30 = 30,
    Unknown31 = 31,
    Unknown32 = 32,
    Unknown33 = 33,
    Unknown34 = 34,
    Unknown35 = 35,
    Unknown36 = 36,
    Unknown37 = 37,
    Unknown38 = 38,
    Unknown39 = 39,
    Unknown40 = 40,
    Unknown41 = 41,
    Unknown42 = 42,
    Unknown43 = 43,
    Undefined = 0x7F,
}

impl From<u16> for ValueType {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Integer,
            2 => Self::Integer64,
            3 => Self::Real,
            4 => Self::String,
            5 => Self::GuidString,
            6 => Self::CharacterGuid,
            7 => Self::ItemGuid,
            8 => Self::Unknown8,
            9 => Self::Unknown9,
            10 => Self::Unknown10,
            11 => Self::Unknown11,
            12 => Self::Unknown12,
            13 => Self::Unknown13,
            14 => Self::Unknown14,
            15 => Self::Unknown15,
            16 => Self::Unknown16,
            17 => Self::Unknown17,
            18 => Self::Unknown18,
            19 => Self::Unknown19,
            20 => Self::Unknown20,
            21 => Self::Unknown21,
            22 => Self::Unknown22,
            23 => Self::Unknown23,
            24 => Self::Unknown24,
            25 => Self::Unknown25,
            26 => Self::Unknown26,
            27 => Self::Unknown27,
            28 => Self::Unknown28,
            29 => Self::Unknown29,
            30 => Self::Unknown30,
            31 => Self::Unknown31,
            32 => Self::Unknown32,
            33 => Self::Unknown33,
            34 => Self::Unknown34,
            35 => Self::Unknown35,
            36 => Self::Unknown36,
            37 => Self::Unknown37,
            38 => Self::Unknown38,
            39 => Self::Unknown39,
            40 => Self::Unknown40,
            41 => Self::Unknown41,
            42 => Self::Unknown42,
            43 => Self::Unknown43,
            0x7F => Self::Undefined,
            x => {
                warn!("unknown Osiris function type {x}");
                Self::Undefined
            }
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionDb {
    pub hash: [HashSlot; 1023],
    pub num_items: u32,

    pub function_id_hash: [FunctionIdHash; 1023],
    pub all_function_ids: TMap<u32, u32>,
    u1: *const (),
    u2: u32,
    u3: [*const (); 8],
}

impl FunctionDb {
    pub fn find(&self, hash: u32, key: &OsiStr) -> Option<GamePtr<GamePtr<Function>>> {
        let bucket = &self.hash[(hash % 0x3FF) as usize];
        bucket.node_map.find(key)
    }

    pub fn find_by_id(&self, id: &u32) -> Option<GamePtr<*const ()>> {
        let bucket = &self.function_id_hash[(id % 0x3FF) as usize];
        bucket.node_map.find(id)
    }

    pub fn functions(&self) -> impl Iterator<Item = (&OsiStr, &Function)> {
        self.hash.iter().flat_map(|x| x.node_map.iter()).map(|x| (&x.kv.key, x.kv.value.as_ref()))
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct HashSlot {
    node_map: TMap<OsiStr, GamePtr<Function>>,
    unknown: *const (),
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionIdHash {
    node_map: TMap<u32, *const ()>,
    unknown: *const (),
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Function {
    pub vptr: *const (),
    pub line: u32,
    pub unknown1: u32,
    pub unknown2: u32,
    pub signature: GamePtr<FunctionSignature>,
    pub node: NodeRef,
    pub r#type: FunctionType,
    pub key: [u32; 4],
    pub osi_function_id: u32,
}

impl Function {
    pub fn handle(&self) -> u32 {
        let r#type = self.key[0];
        let part2 = self.key[1];
        let function_id = self.key[2];
        let part4 = self.key[3];

        let mut handle = (r#type & 7) | (part4 << 31);
        if r#type < 4 {
            handle |= (function_id & 0x1FFFFFF) << 3;
        } else {
            handle |= ((function_id & 0x1FFFF) << 3) | ((part2 & 0xFF) << 20);
        }

        handle
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionSignature {
    pub vptr: *const (),
    pub name: *const (),
    pub params: GamePtr<FunctionParamList>,
    pub out_param_list: FuncSigOutParamList,
    pub unknown: u32,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionParamList {
    pub vptr: *const (),
    pub params: List<FunctionParamDesc>,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionParamDesc {
    pub r#type: u16,
    pub unknown: u32,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FuncSigOutParamList {
    pub params: *const u8,
    pub count: u32,
}

impl FuncSigOutParamList {
    pub fn num_out_params(&self) -> u32 {
        let mut n_params = 0;

        for i in 0..self.count {
            n_params += unsafe { (*self.params.add(i as usize)).count_ones() };
        }

        n_params
    }

    pub fn is_out_param(&self, i: usize) -> bool {
        assert!(i < (self.count * 8) as usize);
        unsafe { (((*self.params.add(i >> 3)) << (i & 7)) & 0b1000_0000) == 0b1000_0000 }
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct NodeRef {
    pub id: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub(crate) enum FunctionType {
    Unknown = 0,
    Event = 1,
    Query = 2,
    Call = 3,
    Database = 4,
    Proc = 5,
    SysQuery = 6,
    SysCall = 7,
    UserQuery = 8,
}

impl Display for FunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionType::Unknown => f.write_str("Unknown"),
            FunctionType::Event => f.write_str("Event"),
            FunctionType::Query => f.write_str("Query"),
            FunctionType::Call => f.write_str("Call"),
            FunctionType::Database => f.write_str("Database"),
            FunctionType::Proc => f.write_str("Procedure"),
            FunctionType::SysQuery => f.write_str("System Query"),
            FunctionType::SysCall => f.write_str("System Call"),
            FunctionType::UserQuery => f.write_str("User Query"),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct TMap<K: PartialOrd, V> {
    root: GamePtr<TMapNode<K, V>>,
}

impl<K: PartialOrd, V> TMap<K, V> {
    pub fn find(&self, key: &K) -> Option<GamePtr<V>> {
        let mut final_tree_node = self.root;
        let mut current_tree_node = self.root.root;
        while !current_tree_node.is_root {
            if current_tree_node.kv.key < *key {
                current_tree_node = current_tree_node.right;
            } else {
                final_tree_node = current_tree_node;
                current_tree_node = current_tree_node.left;
            }
        }

        if final_tree_node == self.root || *key < final_tree_node.kv.key {
            None
        } else {
            Some((&final_tree_node.kv.value).into())
        }
    }

    pub fn iter(&self) -> TMapIter<'_, K, V> {
        self.into_iter()
    }
}

impl<'a, K: PartialOrd, V> IntoIterator for &'a TMap<K, V> {
    type IntoIter = TMapIter<'a, K, V>;
    type Item = &'a TMapNode<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        TMapIter { nodes: vec![&self.root.root] }
    }
}

#[derive(Debug)]
pub(crate) struct TMapIter<'a, K: PartialOrd, V> {
    nodes: Vec<&'a TMapNode<K, V>>,
}

impl<'a, K: PartialOrd, V> Iterator for TMapIter<'a, K, V> {
    type Item = &'a TMapNode<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut last = self.nodes.pop()?;
        while last.is_root {
            last = self.nodes.pop()?;
        }
        self.nodes.push(last.right.as_ref());
        self.nodes.push(last.left.as_ref());
        Some(last)
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TMapNode<K, V> {
    left: GamePtr<TMapNode<K, V>>,
    root: GamePtr<TMapNode<K, V>>,
    right: GamePtr<TMapNode<K, V>>,
    color: bool,
    is_root: bool,
    kv: KeyValuePair<K, V>,
}

#[derive(Debug)]
#[repr(C)]
struct KeyValuePair<K, V> {
    key: K,
    value: V,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct List<T> {
    pub head: GamePtr<ListNode<T>>,
    pub size: u64,
}

impl<'a, T> IntoIterator for &'a List<T> {
    type IntoIter = ListIter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        ListIter { len: self.size as usize, next: self.head.next, _marker: Default::default() }
    }
}

impl<T> List<T> {
    pub fn iter(&self) -> ListIter<'_, T> {
        self.into_iter()
    }
}

pub(crate) struct ListIter<'a, T> {
    len: usize,
    next: GamePtr<ListNode<T>>,
    _marker: marker::PhantomData<&'a List<T>>,
}

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let v = self.next.as_ref();
        self.next = self.next.next;
        Some(&v.item)
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ListNode<T> {
    pub next: GamePtr<ListNode<T>>,
    pub head: GamePtr<ListNode<T>>,
    pub item: T,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct OsiString {
    pub str: OsiStr,
}

impl OsiString {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 16 {
            let mut buf = [0u8; 16];
            buf[..bytes.len()].clone_from_slice(bytes);
            Self { str: OsiStr { ptr_or_buf: PtrOrBuf { buf }, size: bytes.len(), capacity: 15 } }
        } else {
            let ptr = unsafe {
                let layout = alloc::Layout::from_size_align_unchecked(bytes.len() + 1, 1);
                let ptr = alloc::alloc(layout);
                if ptr.is_null() {
                    alloc::handle_alloc_error(layout);
                }
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
                ptr.add(bytes.len()).write(0);
                ptr
            };

            Self {
                str: OsiStr {
                    ptr_or_buf: PtrOrBuf { ptr },
                    size: bytes.len(),
                    capacity: bytes.len(),
                },
            }
        }
    }
}

impl Drop for OsiString {
    fn drop(&mut self) {
        if self.str.is_large_mode() {
            unsafe {
                alloc::dealloc(
                    self.str.ptr_or_buf.ptr,
                    alloc::Layout::from_size_align_unchecked(self.str.capacity + 1, 1),
                );
            }
        }
    }
}

#[repr(C)]
pub(crate) struct OsiStr {
    ptr_or_buf: PtrOrBuf,
    size: usize,
    capacity: usize,
}

impl Debug for OsiStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsiString")
            .field("ptr_or_buf", &self.as_cstr())
            .field("size", &self.size)
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl Display for OsiStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq for OsiStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_cstr().eq(other.as_cstr())
    }
}

impl PartialOrd for OsiStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_cstr().partial_cmp(other.as_cstr())
    }
}

impl OsiStr {
    pub fn as_str(&self) -> &str {
        self.as_cstr().to_str().expect("OsiStirng conatains invalid UTF-8 data")
    }

    pub fn as_cstr(&self) -> &CStr {
        if self.is_large_mode() {
            unsafe { CStr::from_ptr(self.ptr_or_buf.ptr as _) }
        } else {
            unsafe { CStr::from_ptr(self.ptr_or_buf.buf.as_ptr() as _) }
        }
    }

    fn is_large_mode(&self) -> bool {
        self.capacity > 15
    }
}
