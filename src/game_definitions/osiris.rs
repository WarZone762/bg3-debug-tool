use std::{alloc, ffi::CStr, fmt::Debug, marker};

use super::{GamePtr, PtrOrBuf};

#[derive(Clone, Copy, Debug)]
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
pub(crate) struct OsiArgumentDesc {
    pub next_param: GamePtr<Self>,
    pub value: OsiArgumentValue,
}

impl OsiArgumentDesc {
    pub fn from_value(value: OsiArgumentValue) -> GamePtr<Self> {
        Box::leak(Box::new(Self { next_param: GamePtr::null(), value })).into()
    }

    pub fn from_values(mut iter: impl Iterator<Item = OsiArgumentValue>) -> GamePtr<Self> {
        let first = Self::from_value(iter.next().unwrap_or(OsiArgumentValue::undefined()));
        let mut last = first;

        for e in iter {
            let value = Self::from_value(e);
            last.next_param = value;
            last = value;
        }

        first
    }
}

#[derive(Default)]
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
                    "CharacterGUid({})",
                    CStr::from_ptr(self.value.string as _).to_str().unwrap()
                )
            },
            ValueType::ItemGuid => unsafe {
                format!("ItemGuid({})", CStr::from_ptr(self.value.string as _).to_str().unwrap())
            },
            ValueType::Undefined => "Undefined".into(),
        };

        f.debug_struct("OsiArgumentValue")
            .field("value", &value)
            .field("value_type", &self.type_id)
            .field("unknown", &self.unknown)
            .finish()
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

    pub fn undefined() -> Self {
        Self { type_id: ValueType::Undefined, ..Default::default() }
    }
}

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    Undefined = 0x7F,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionDb {
    hash: [HashSlot; 1023],
    num_items: u32,

    function_id_hash: [FunctionIdHash; 1023],
    all_function_ids: TMap<u32, u32>,
    u1: *const (),
    u2: u32,
    u3: [*const (); 8],
}

impl FunctionDb {
    pub fn find(&self, hash: u32, key: &OsiString) -> Option<GamePtr<GamePtr<Function>>> {
        let bucket = &self.hash[(hash % 0x3FF) as usize];
        bucket.node_map.find(key)
    }

    pub fn find_by_id(&self, id: &u32) -> Option<GamePtr<*const ()>> {
        let bucket = &self.function_id_hash[(id % 0x3FF) as usize];
        bucket.node_map.find(id)
    }
}

#[repr(C)]
pub(crate) struct OsiStringOwned {
    pub string: OsiString,
}

impl OsiStringOwned {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 16 {
            let mut buf = [0u8; 16];
            buf[..bytes.len()].clone_from_slice(bytes);
            Self {
                string: OsiString { ptr_or_buf: PtrOrBuf { buf }, size: bytes.len(), capacity: 15 },
            }
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
                string: OsiString {
                    ptr_or_buf: PtrOrBuf { ptr },
                    size: bytes.len(),
                    capacity: bytes.len(),
                },
            }
        }
    }
}

impl Drop for OsiStringOwned {
    fn drop(&mut self) {
        if self.string.is_large_mode() {
            unsafe {
                alloc::dealloc(
                    self.string.ptr_or_buf.ptr,
                    alloc::Layout::from_size_align_unchecked(self.string.capacity + 1, 1),
                );
            }
        }
    }
}

#[repr(C)]
pub(crate) struct OsiString {
    ptr_or_buf: PtrOrBuf,
    size: usize,
    capacity: usize,
}

impl Debug for OsiString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsiString")
            .field("ptr_or_buf", &self.c_str())
            .field("size", &self.size)
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl PartialEq for OsiString {
    fn eq(&self, other: &Self) -> bool {
        self.c_str().eq(other.c_str())
    }
}

impl PartialOrd for OsiString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.c_str().partial_cmp(other.c_str())
    }
}

impl OsiString {
    pub fn c_str(&self) -> &CStr {
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

#[derive(Debug)]
#[repr(C)]
struct HashSlot {
    node_map: TMap<OsiString, GamePtr<Function>>,
    unknown: *const (),
}

#[derive(Debug)]
#[repr(C)]
struct FunctionIdHash {
    node_map: TMap<u32, *const ()>,
    unknown: *const (),
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Function {
    pub vmt: *const (),
    pub line: u32,
    pub unknown1: u32,
    pub unknown2: u32,
    pub signatrue: GamePtr<FunctionSignature>,
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
    pub vmt: *const (),
    pub name: *const (),
    pub params: GamePtr<FunctionParamList>,
    pub out_param_list: FuncSigOutParamList,
    pub unknown: u32,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionParamList {
    pub vmt: *const (),
    pub params: List<FunctionParamDesc>,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionParamDesc {
    pub r#type: ValueType,
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

#[derive(Debug)]
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
}

#[derive(Debug)]
#[repr(C)]
struct TMapNode<K, V> {
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
