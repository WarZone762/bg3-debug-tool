use std::{alloc, ffi::CStr, fmt::Debug};

#[derive(Debug)]
#[repr(C)]
pub(crate) struct OsiArgumentDesc {
    next_param: *mut Self,
    pub value: OsiArgumentValue,
}

impl OsiArgumentDesc {
    pub fn from_value(value: OsiArgumentValue) -> *mut Self {
        Box::leak(Box::new(Self {
            next_param: std::ptr::null_mut(),
            value,
        })) as _
    }

    pub fn from_values(mut iter: impl Iterator<Item = OsiArgumentValue>) -> *mut Self {
        let first = Self::from_value(iter.next().unwrap_or(OsiArgumentValue::undefined()));
        let mut last = first;

        for e in iter {
            let value = Self::from_value(e);
            unsafe { (*last).next_param = value };
            last = value;
        }

        first
    }
}

#[repr(C)]
pub(crate) struct OsiArgumentValue {
    value: OsiArgumentValueUnion,
    value_type: TypeId,
    unknown: bool,
}

impl Debug for OsiArgumentValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self.value_type {
            TypeId::None => "None".into(),
            TypeId::Integer => unsafe { self.value.int32.to_string() },
            TypeId::Integer64 => unsafe { self.value.int64.to_string() },
            TypeId::Real => unsafe { self.value.float.to_string() },
            TypeId::String => {
                unsafe { CStr::from_ptr(self.value.string as _).to_str().unwrap() }.to_string()
            }
            TypeId::GuidString => {
                unsafe { CStr::from_ptr(self.value.string as _).to_str().unwrap() }.to_string()
            }
            TypeId::Undefined => "Undefined".into(),
        };

        f.debug_struct("OsiArgumentValue")
            .field("value", &value)
            .field("value_type", &self.value_type)
            .field("unknown", &self.unknown)
            .finish()
    }
}

impl OsiArgumentValue {
    pub fn none() -> Self {
        Self {
            value: OsiArgumentValueUnion { int64: 0 },
            value_type: TypeId::None,
            unknown: false,
        }
    }

    pub fn int32(int32: i32) -> Self {
        Self {
            value: OsiArgumentValueUnion { int32 },
            value_type: TypeId::Integer,
            unknown: false,
        }
    }

    pub fn int64(int64: i64) -> Self {
        Self {
            value: OsiArgumentValueUnion { int64 },
            value_type: TypeId::Integer64,
            unknown: false,
        }
    }

    pub fn real(float: f32) -> Self {
        Self {
            value: OsiArgumentValueUnion { float },
            value_type: TypeId::Integer64,
            unknown: false,
        }
    }

    pub fn string(string: *const i8) -> Self {
        Self {
            value: OsiArgumentValueUnion { string },
            value_type: TypeId::String,
            unknown: false,
        }
    }

    pub fn guid_string(string: *const i8) -> Self {
        Self {
            value: OsiArgumentValueUnion { string },
            value_type: TypeId::GuidString,
            unknown: false,
        }
    }

    pub fn undefined() -> Self {
        Self {
            value: OsiArgumentValueUnion { int64: 0 },
            value_type: TypeId::Undefined,
            unknown: false,
        }
    }
}

#[repr(C)]
union OsiArgumentValueUnion {
    string: *const i8,
    int32: i32,
    int64: i64,
    float: f32,
}

#[derive(Debug)]
#[repr(u16)]
enum TypeId {
    None = 0,
    Integer = 1,
    Integer64 = 2,
    Real = 3,
    String = 4,
    GuidString = 5,
    Undefined = 0x7f,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FunctionDb {
    hash: [HashSlot; 1023],
    num_items: u32,

    function_id_hash: [FunctionIdHash; 1023],
    all_function_ids: TMap<u32, u32>,
    u1: *const u8,
    u2: u32,
    u3: [*const u8; 8],
}

impl FunctionDb {
    pub fn find(&self, hash: u32, key: OsiString) -> Option<*const *const Function> {
        let bucket = &self.hash[(hash % 0x3FF) as usize];
        bucket.node_map.find(key)
    }

    pub fn find_by_id(&self, id: u32) -> Option<*const *const u8> {
        let bucket = &self.function_id_hash[(id % 0x3FF) as usize];
        bucket.node_map.find(id)
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
        self.c_str().fmt(f)
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

impl Drop for OsiString {
    fn drop(&mut self) {
        if self.is_large_mode() {
            unsafe {
                alloc::dealloc(
                    self.ptr_or_buf.ptr,
                    alloc::Layout::from_size_align_unchecked(self.capacity + 1, 1),
                );
            }
        }
    }
}

impl OsiString {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 16 {
            let mut buf = [0u8; 16];
            buf[..bytes.len()].clone_from_slice(bytes);
            Self {
                ptr_or_buf: PtrOrBuf { buf },
                size: bytes.len(),
                capacity: 15,
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
                ptr_or_buf: PtrOrBuf { ptr },
                size: bytes.len(),
                capacity: bytes.len(),
            }
        }
    }

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

#[repr(C)]
union PtrOrBuf {
    ptr: *mut u8,
    buf: [u8; 16],
}

#[derive(Debug)]
#[repr(C)]
struct HashSlot {
    node_map: TMap<OsiString, *const Function>,
    unknown: *const u8,
}

#[derive(Debug)]
#[repr(C)]
struct FunctionIdHash {
    node_map: TMap<u32, *const u8>,
    unknown: *const u8,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Function {
    vmt: *const (),
    line: u32,
    unknown1: u32,
    unknown2: u32,
    signatrue: *const FunctionSignature,
    node: NodeRef,
    r#type: FunctionType,
    key: [u32; 4],
    osi_function_id: u32,
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
struct FunctionSignature {
    vmt: *const (),
    name: *const u8,
    params: *const FunctionParamList,
    out_param_list: FuncSigOutParamList,
    unknown: u32,
}

#[derive(Debug)]
#[repr(C)]
struct FunctionParamList {}

#[derive(Debug)]
#[repr(C)]
struct FuncSigOutParamList {
    params: *const u8,
    count: u32,
}

#[derive(Debug)]
#[repr(C)]
struct NodeRef {
    id: u32,
    manager: *const (),
}

#[derive(Debug)]
#[repr(u32)]
enum FunctionType {
    Unknown = 0,
}

#[derive(Debug)]
#[repr(C)]
struct TMap<K: PartialOrd, V> {
    root: *mut TMapNode<K, V>,
}

impl<K: PartialOrd, V> TMap<K, V> {
    pub fn find(&self, key: K) -> Option<*const V> {
        let mut final_tree_node = self.root;
        let mut current_tree_node = unsafe { (*self.root).root };
        while !unsafe { (*current_tree_node).is_root } {
            if unsafe { (*current_tree_node).kv.key < key } {
                current_tree_node = unsafe { (*current_tree_node).right };
            } else {
                final_tree_node = current_tree_node;
                current_tree_node = unsafe { (*current_tree_node).left };
            }
        }

        if final_tree_node == self.root || unsafe { key < (*final_tree_node).kv.key } {
            None
        } else {
            Some(unsafe { &(*final_tree_node).kv.value })
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct TMapNode<K, V> {
    left: *mut TMapNode<K, V>,
    root: *mut TMapNode<K, V>,
    right: *mut TMapNode<K, V>,
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
