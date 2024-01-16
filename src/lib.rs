#![feature(let_chains, c_variadic, try_trait_v2, never_type)]

pub mod base_utilities;
mod binary_mappings;
pub mod console;
mod script_extender;
mod shims;

use std::{
    ffi::CStr,
    fmt::Debug,
    io::{BufRead, Read},
    os::windows::io::AsRawHandle,
};

use binary_mappings::{xml, SymbolMapper};
use widestring::u16cstr;
use windows::{
    core::{s, w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, BOOL, HANDLE, HMODULE},
        System::{
            LibraryLoader::{GetProcAddress, LoadLibraryW},
            SystemInformation::GetSystemDirectoryW,
            Threading::GetCurrentThread,
        },
    },
};

use crate::script_extender::LibraryManager;

pub(crate) const BINARY_MAPPINGS_XML: &str = include_str!("BinaryMappings.xml");

pub(crate) static mut IO: Io = Io::None;

#[derive(Debug)]
pub(crate) enum Io {
    None,
    Normal(std::io::StdinLock<'static>, std::io::Stdout),
    Tcp(std::io::BufReader<std::net::TcpStream>, std::net::TcpStream),
}

unsafe impl Sync for Io {}

impl std::io::Read for Io {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Io::None => Ok(0),
            Io::Normal(stdin, _) => stdin.read(buf),
            Io::Tcp(s, _) => s.read(buf),
        }
    }
}

impl std::io::BufRead for Io {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        match self {
            Io::None => Ok(&[]),
            Io::Normal(stdin, _) => stdin.fill_buf(),
            Io::Tcp(s, _) => s.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            Io::None => (),
            Io::Normal(stdin, _) => stdin.consume(amt),
            Io::Tcp(s, _) => s.consume(amt),
        }
    }
}

impl std::io::Write for Io {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Io::None => Ok(0),
            Io::Normal(_, stdout) => stdout.write(buf),
            Io::Tcp(_, s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Io::None => Ok(()),
            Io::Normal(_, stdout) => stdout.flush(),
            Io::Tcp(_, s) => s.flush(),
        }
    }
}

impl Io {
    pub fn normal() {
        unsafe { IO = Io::Normal(std::io::stdin().lock(), std::io::stdout()) }
    }

    pub fn tcp(addr: impl std::net::ToSocketAddrs) {
        let listener = std::net::TcpListener::bind(addr).unwrap();
        let c = listener.accept().unwrap().0;

        unsafe {
            IO = Io::Tcp(std::io::BufReader::new(c.try_clone().unwrap()), c);
        }
    }

    pub fn get() -> &'static mut Self {
        unsafe { &mut IO }
    }
}

#[macro_export]
macro_rules! info {
    ($($tt:tt)*) => {
        $crate::_print!("\x1b[1m");
        $crate::_print!($($tt)*);
        $crate::_println!("\x1b[0m")
    };
}

#[macro_export]
macro_rules! warn {
    ($($tt:tt)*) => {
        $crate::_print!("\x1b[33m");
        $crate::_print!($($tt)*);
        $crate::_println!("\x1b[0m")
    };
}

#[macro_export]
macro_rules! err {
    ($($tt:tt)*) => {
        $crate::_print!("\x1b[31m");
        $crate::_print!($($tt)*);
        $crate::_println!("\x1b[0m")
    };
}

#[macro_export]
macro_rules! _print {
    ($($tt:tt)*) => {
        {
            use std::io::Write;
            write!(unsafe {&mut $crate::IO}, $($tt)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! _println {
    ($($tt:tt)*) => {
        {
            use std::io::Write;
            writeln!(unsafe {&mut $crate::IO}, $($tt)*).unwrap();
        }
    };
}

#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum DllCallReason {
    DLL_PROCESS_ATTACH = 1,
    DLL_PROCESS_DETACH = 0,
    DLL_THREAD_ATTACH = 2,
    DLL_THREAD_DETACH = 3,
}

fn load_dwrite() -> windows::core::Result<HMODULE> {
    let mut dll_path = [0; 2048];

    unsafe {
        let path_size = GetSystemDirectoryW(Some(&mut dll_path)) as usize;
        if path_size == 0 {
            return Err(GetLastError().unwrap_err());
        }

        let dll_name = u16cstr!("\\DWrite.dll");
        dll_path[path_size..(path_size + dll_name.len())].copy_from_slice(dll_name.as_slice());

        LoadLibraryW(PCWSTR(dll_path.as_ptr()))
    }
}

#[no_mangle]
pub extern "system" fn DllMain(_dll: HANDLE, reason: DllCallReason, _reserved: &u32) -> BOOL {
    match reason {
        DllCallReason::DLL_PROCESS_ATTACH => main(),
        DllCallReason::DLL_PROCESS_DETACH => (),
        _ => (),
    }
    true.into()
}

static mut OSIRIS_GLOBALS: Option<OsirisStaticGlobals> = None;

// static mut RegisterDivFunctions: fn(*const u8, *const u8) -> i32 = std::ptr::null();
static mut RegisterDivFunctions: *const libc::c_void = std::ptr::null();
static mut CALL: *const libc::c_void = std::ptr::null();
static mut QUERY: *const libc::c_void = std::ptr::null();

extern "C" fn RegisterDivFunctionsHook(a: *const u8, b: *const u8) -> i32 {
    unsafe {
        let osiris_module = LoadLibraryW(w!("Osiris.dll")).unwrap();
        let osiris_ctor_proc =
            GetProcAddress(osiris_module, s!("??0COsiris@@QEAA@XZ")).unwrap() as _;

        OSIRIS_GLOBALS = find_osiris_globals(osiris_ctor_proc);

        CALL = *(b as *const usize).add(1) as _;
        QUERY = *(b as *const usize).add(2) as _;

        std::mem::transmute::<_, extern "C" fn(*const u8, *const u8) -> i32>(RegisterDivFunctions)(
            a, b,
        )
    }
}

#[link(name = "detours", kind = "static")]
extern "C" {
    fn DetourTransactionBegin();
    fn DetourUpdateThread(handle: HANDLE);
    fn DetourAttach(
        ppPointer: *const *const libc::c_void,
        pDetour: *const libc::c_void,
    ) -> libc::c_long;
    fn DetourTransactionCommit();
}

fn main() {
    let use_tcp = true;
    if use_tcp {
        Io::tcp("127.0.0.1:9003");
    } else {
        Io::normal();
    }

    load_dwrite().unwrap();

    if let Ok(version) = LibraryManager::game_version() {
        if version.is_supported() {
            info!("Game version {version} OK");
        } else {
            err!("Game versino {version} is not supported, please upgrade!");
            panic!("Scrip Extender doesn't support game versions below v4.37, please upgrade!");
        }
    } else {
        err!("Failed to retrieve game version info.");
    }

    // let binary_mappings: xml::BinaryMappings =
    //     quick_xml::de::from_str(BINARY_MAPPINGS_XML).unwrap();
    // let mut symbol_mapper = SymbolMapper::new().unwrap();
    //
    // symbol_mapper.populate_mappings(binary_mappings.try_into().unwrap());
    // info!("{symbol_mapper:#?}");

    // unsafe {
    //     let osiris_module = LoadLibraryW(w!("Osiris.dll")).unwrap();
    //     let osiris_ctor_proc =
    //         GetProcAddress(osiris_module, s!("??0COsiris@@QEAA@XZ")).unwrap() as _;
    //     info!("{osiris_ctor_proc:?}");
    //
    //     find_osiris_globals(osiris_ctor_proc);
    // }

    unsafe {
        let osiris_module = LoadLibraryW(w!("Osiris.dll")).unwrap();
        let tgt = GetProcAddress(
            osiris_module,
            s!("?RegisterDIVFunctions@COsiris@@QEAAXPEAUTOsirisInitFunction@@@Z"),
        )
        .unwrap();
        RegisterDivFunctions = tgt as _;
        DetourTransactionBegin();
        DetourUpdateThread(GetCurrentThread());
        DetourAttach(&RegisterDivFunctions as _, RegisterDivFunctionsHook as _);
        DetourTransactionCommit();
    }

    type CallProc = extern "C" fn(function_handle: u32, params: *const OsiArgumentDesc) -> bool;

    #[repr(C)]
    struct OsiArgumentDesc {
        next_param: *const Self,
        value: OsiArgumentValue,
    }

    impl OsiArgumentDesc {
        pub fn from_value(value: OsiArgumentValue) -> *const Self {
            Box::leak(Box::new(Self {
                next_param: std::ptr::null(),
                value,
            })) as _
        }

        pub fn from_values(mut iter: impl Iterator<Item = OsiArgumentValue>) -> *const Self {
            let first = Self::from_value(iter.next().unwrap_or(OsiArgumentValue::undefined()));
            let mut last = first as *mut Self;

            for e in iter {
                let value = Self::from_value(e);
                unsafe { (*last).next_param = value };
                last = value as *mut _;
            }

            first
        }
    }

    #[repr(C)]
    struct OsiArgumentValue {
        value: OsiArgumentValueUnion,
        value_type: TypeId,
        unknown: bool,
    }

    impl OsiArgumentValue {
        pub fn none() -> Self {
            Self {
                value: OsiArgumentValueUnion { int64: 0 },
                value_type: TypeId::None,
                unknown: false,
            }
        }

        pub fn string(string: *const u8) -> Self {
            Self {
                value: OsiArgumentValueUnion { string },
                value_type: TypeId::String,
                unknown: false,
            }
        }

        pub fn guid_string(string: *const u8) -> Self {
            Self {
                value: OsiArgumentValueUnion { string },
                value_type: TypeId::GuidString,
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
        string: *const u8,
        int32: i32,
        int64: i64,
        float: f32,
    }

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

    std::thread::spawn(|| {
        let mut buf = String::new();
        loop {
            _print!(">> ");
            let _ = Io::get().read_line(&mut buf);
            let func = buf.trim();
            info!("searching for '{func}'");

            if let Some(globals) = &unsafe { OSIRIS_GLOBALS } {
                let name = OsiString::from_bytes(b"GetHostCharacter/1");
                let hash = function_name_hash(b"GetHostCharacter\0") + 1;
                // let name = OsiString::from_bytes(b"Random/2");
                // let hash = function_name_hash(b"Random\0") + 2;

                let res = unsafe { (**globals.functions).find(hash, name) };
                info!("{res:?}");

                if let Some(res) = res {
                    let res = unsafe { (**res).handle() };
                    // let call = unsafe { std::mem::transmute::<_, CallProc>(CALL) };
                    let query = unsafe { std::mem::transmute::<_, CallProc>(QUERY) };
                    // let buf = [0u8; 128];
                    let out = OsiArgumentDesc::from_value(OsiArgumentValue::guid_string(
                        std::ptr::null(),
                    ));
                    unsafe {
                        print_bytes(std::slice::from_raw_parts(out as _, 24), 8);
                    }
                    // let modulo = OsiArgumentValue::int32(100);
                    // let out = OsiArgumentValue::int64(0);

                    // let args = OsiArgumentDesc::from_values([modulo, out].into_iter());
                    // let arg2 = OsiArgumentDesc {
                    //     next_param: std::ptr::null(),
                    //     value: OsiArgumentValue::int32(100),
                    // };
                    // let arg1 = OsiArgumentDesc {
                    //     next_param: &arg2 as _,
                    //     value: OsiArgumentValue::int32(100),
                    // };

                    query(res, out);

                    // info!("{buf:?}");
                    unsafe {
                        print_bytes(std::slice::from_raw_parts(out as _, 24), 8);
                    }

                    _println!("{:?}", unsafe {
                        CStr::from_ptr((*out).value.value.string as _)
                    });

                    // unsafe {
                    //     print_bytes(std::slice::from_raw_parts((*out).value.value.string, 32), 8);
                    // }
                    // unsafe {
                    //     print_bytes(std::slice::from_raw_parts(&arg1 as *const _ as _, 24), 8);
                    // }
                    // unsafe {
                    //     print_bytes(std::slice::from_raw_parts(arg1.next_param as _, 24), 8);
                    // }
                }

                // let mut fn_addrs = [std::ptr::null::<*const u8>(); 6];
                // for i in 0..7 {
                //     let name = OsiString::from_bytes(format!("{func}/{i}").as_bytes());
                //     let hash = function_name_hash(format!("{func}\0").as_bytes()) + i;
                //
                //     // let name = OsiString::from_bytes(b"AddExplorationExperience/2");
                //     // let hash = function_name_hash(b"AddExplorationExperience\0") + 2;
                //
                //     let res = unsafe { (**globals.functions).find(hash, name) };
                //     if let Some(res) = res {
                //         info!("{i} args: {res:#?}");
                //         fn_addrs[i as usize] = res;
                //     }
                // }
            }
            buf.clear();
        }
    });

    info!("Test");
    warn!("Test");
    err!("Test");
    _println!("Test");
}

#[derive(Clone, Copy, Debug)]
struct OsirisStaticGlobals {
    variables: *const *const u8,
    types: *const *const u8,
    enums: *const *const u8,
    functions: *const *const FunctionDb,
    objects: *const *const u8,
    goals: *const *const u8,
    adapters: *const *const u8,
    databases: *const *const u8,
    nodes: *const *const u8,
}

impl OsirisStaticGlobals {
    pub fn new() -> Self {
        Self {
            variables: std::ptr::null(),
            types: std::ptr::null(),
            enums: std::ptr::null(),
            functions: std::ptr::null(),
            objects: std::ptr::null(),
            goals: std::ptr::null(),
            adapters: std::ptr::null(),
            databases: std::ptr::null(),
            nodes: std::ptr::null(),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct FunctionDb {
    hash: [HashSlot; 1023],
    num_items: u32,

    function_id_hash: [FunctionIdHash; 1023],
    all_function_ids: TMap<u32, u32>,
    u1: *const u8,
    u2: u32,
    u3: [*const u8; 8],
}

unsafe fn print_bytes(buf: &[u8], width: usize) {
    let mut chars = String::with_capacity(width);
    let mut bytes = String::with_capacity(width * 3);

    for (i, b) in buf.iter().enumerate() {
        let c = *b as char;

        if c.is_ascii_graphic() {
            chars.push(c);
        } else {
            chars.push('.');
        }

        bytes.push_str(&format!("{:02X}", c as u8));

        if (i + 1) % width == 0 {
            info!("{bytes}    {chars}");
            chars.clear();
            bytes.clear();
        } else {
            bytes.push(' ');
        }
    }

    if buf.len() % width != 0 {
        info!("{bytes}    {chars}");
    }
}

impl FunctionDb {
    pub fn find(&self, hash: u32, key: OsiString) -> Option<*const *const Function> {
        let bucket = &self.hash[(hash % 0x3FF) as usize];
        unsafe { bucket.node_map.find(key) }
    }

    pub fn find_by_id(&self, id: u32) -> Option<*const *const u8> {
        let bucket = &self.function_id_hash[(id % 0x3FF) as usize];
        unsafe { bucket.node_map.find(id) }
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

#[repr(C)]
union PtrOrBuf {
    ptr: *const u8,
    buf: [u8; 16],
}

#[repr(C)]
struct OsiString {
    ptr_or_buf: PtrOrBuf,
    size: usize,
    capacity: usize,
}

impl Debug for OsiString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.c_str().fmt(f)
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
                let layout = std::alloc::Layout::from_size_align(bytes.len() + 1, 1).unwrap();
                let ptr = std::alloc::alloc(layout);
                if ptr.is_null() {
                    std::alloc::handle_alloc_error(layout);
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
        if self.capacity > 15 {
            unsafe { CStr::from_ptr(self.ptr_or_buf.ptr as _) }
        } else {
            unsafe { CStr::from_ptr(self.ptr_or_buf.buf.as_ptr() as _) }
        }
    }
}

#[derive(Debug)]
#[repr(C)]
struct HashSlot {
    node_map: TMap<OsiString, *const Function, PredicateStrcmp>,
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
struct Function {
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
struct TMap<K, V, Pred: Predicate<K> = PredicateLess> {
    root: *const TMapNode<K, V>,
    _pred: std::marker::PhantomData<Pred>,
}

impl<K, V, Pred: Predicate<K>> TMap<K, V, Pred> {
    pub unsafe fn find(&self, key: K) -> Option<*const V> {
        let mut final_tree_node = self.root;
        let mut current_tree_node = (*self.root).root;
        while !(*current_tree_node).is_root {
            if Pred::compare(&(*current_tree_node).kv.key, &key) {
                current_tree_node = (*current_tree_node).right;
            } else {
                final_tree_node = current_tree_node;
                current_tree_node = (*current_tree_node).left;
            }
        }

        if final_tree_node == self.root || Pred::compare(&key, &(*final_tree_node).kv.key) {
            None
        } else {
            Some(&(*final_tree_node).kv.value)
        }
    }
}

trait Predicate<T> {
    fn compare(a: &T, b: &T) -> bool;
}

#[derive(Debug)]
struct PredicateLess;
impl<T: PartialOrd> Predicate<T> for PredicateLess {
    fn compare(a: &T, b: &T) -> bool {
        a < b
    }
}

#[derive(Debug)]
struct PredicateStrcmp;
impl Predicate<OsiString> for PredicateStrcmp {
    fn compare(a: &OsiString, b: &OsiString) -> bool {
        a.c_str() < b.c_str()
    }
}

#[derive(Debug)]
#[repr(C)]
struct TMapNode<K, V> {
    left: *const TMapNode<K, V>,
    root: *const TMapNode<K, V>,
    right: *const TMapNode<K, V>,
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

fn find_osiris_globals(ctor_proc: *const u8) -> Option<OsirisStaticGlobals> {
    let addr = unsafe { resolve_real_function_address(ctor_proc) };

    let mut globals = [std::ptr::null::<*const u8>(); 9];
    let mut found_globals = 0;

    unsafe {
        for i in 0..0x500 {
            let ptr = addr.add(i);

            if (*ptr == 0x49 || *ptr == 0x48)
                && *ptr.add(1) == 0x8B
                && *ptr.add(3) == 0x48
                && *ptr.add(4) == 0x89
                && (*ptr.add(5) & 0xC7) == 0x05
            {
                let rel_offset = *(ptr.add(6) as *const i32) as isize;
                let osi_ptr = ptr.offset(rel_offset + 10);
                globals[found_globals] = osi_ptr as _;
                found_globals += 1;
                if found_globals == 9 {
                    break;
                }
            }
        }
    }

    if found_globals < 9 {
        err!("Could not locate global Osiris variables");
        return None;
    }

    let osiris_globals = OsirisStaticGlobals {
        variables: globals[0],
        types: globals[1],
        enums: globals[2],
        functions: globals[3] as _,
        objects: globals[4],
        goals: globals[5],
        adapters: globals[6],
        databases: globals[7],
        nodes: globals[8],
    };

    Some(osiris_globals)
}

unsafe fn resolve_real_function_address(ptr: *const u8) -> *const u8 {
    if *ptr == 0xE9 {
        let rel_offset = *(ptr.add(1) as *const i32) as isize;
        return ptr.offset(rel_offset + 5) as _;
    }

    for i in 0..64 {
        let p = ptr.add(i);
        if *p == 0x48
            && *p.add(1) == 0x83
            && *p.add(2) == 0x3D
            && *p.add(6) == 0x00
            && *p.add(13) == 0xE9
        {
            let rel_offset = *(p.add(14) as *const i32) as isize;
            return p.offset(18 + rel_offset) as _;
        }
    }

    ptr
}
