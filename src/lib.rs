#![feature(let_chains, c_variadic, try_trait_v2, never_type, unboxed_closures)]

mod binary_mappings;
mod game_definitions;
mod script_extender;

use std::{cell::OnceCell, io::BufRead, mem, ptr};

use game_definitions::FunctionDb;
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

use crate::{
    game_definitions::{OsiArgumentDesc, OsiArgumentValue, OsiString},
    script_extender::LibraryManager,
};

pub(crate) const BINARY_MAPPINGS_XML: &str = include_str!("BinaryMappings.xml");

pub(crate) static mut STD_IO: OnceCell<StdIo> = OnceCell::new();

#[derive(Debug)]
pub(crate) enum StdIo {
    Normal(std::io::StdinLock<'static>, std::io::Stdout),
    Tcp(std::io::BufReader<std::net::TcpStream>, std::net::TcpStream),
}

impl std::io::Read for StdIo {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            StdIo::Normal(stdin, _) => stdin.read(buf),
            StdIo::Tcp(s, _) => s.read(buf),
        }
    }
}

impl std::io::BufRead for StdIo {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        match self {
            StdIo::Normal(stdin, _) => stdin.fill_buf(),
            StdIo::Tcp(s, _) => s.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            StdIo::Normal(stdin, _) => stdin.consume(amt),
            StdIo::Tcp(s, _) => s.consume(amt),
        }
    }
}

impl std::io::Write for StdIo {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            StdIo::Normal(_, stdout) => stdout.write(buf),
            StdIo::Tcp(_, s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            StdIo::Normal(_, stdout) => stdout.flush(),
            StdIo::Tcp(_, s) => s.flush(),
        }
    }
}

impl StdIo {
    pub fn normal() {
        unsafe {
            STD_IO
                .set(StdIo::Normal(std::io::stdin().lock(), std::io::stdout()))
                .expect("IO already initialized");
        }
    }

    pub fn tcp(addr: impl std::net::ToSocketAddrs) {
        let listener = std::net::TcpListener::bind(addr).unwrap();
        let c = listener.accept().unwrap().0;

        unsafe {
            STD_IO
                .set(StdIo::Tcp(
                    std::io::BufReader::new(c.try_clone().unwrap()),
                    c,
                ))
                .expect("IO already initialized");
        }
    }

    pub fn get() -> &'static mut Self {
        unsafe { STD_IO.get_mut().expect("IO is not initialized") }
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
            write!($crate::StdIo::get(), $($tt)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! _println {
    ($($tt:tt)*) => {
        {
            use std::io::Write;
            #[allow(unused_unsafe)]
            writeln!( $crate::StdIo::get(), $($tt)*).unwrap();
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

static mut HOOKS: Hooks = Hooks::new();

macro_rules! HookDefinitions {
    { $(fn $name: ident($($arg_name: ident: $arg: ty),*) -> $ret: ty $body: block)* } => {
        #[derive(Debug)]
        struct Hooks {
            original: HooksOriginal,
        }

        impl Hooks {
            pub const fn new() -> Self {
                Self {
                    original: HooksOriginal::new(),
                }
            }
        }

        #[allow(non_snake_case, dead_code)]
        mod hooks {
            use super::*;
            $(
                pub extern "C" fn $name($($arg_name: $arg),*) -> $ret $body
            )*
        }

        #[allow(non_snake_case)]
        #[derive(Debug)]
        struct HooksOriginal {
            $(
                $name: HookableFunction<extern "C" fn($($arg_name: $arg),*) -> $ret>,
            )*
        }

        impl HooksOriginal {
            pub const fn new() -> Self {
                Self {
                    $(
                        $name: HookableFunction::new(),
                    )*
                }
            }
        }

        #[allow(non_snake_case, dead_code)]
        mod original {
            use super::*;
            $(
                pub extern "C" fn $name($($arg_name: $arg),*) -> $ret {
                    unsafe { $crate::HOOKS.original.$name.as_ref()($($arg_name),*) }
                }
            )*
        }

    };
}

HookDefinitions! {
    fn RegisterDivFunctions(a: *const u8, b: *const u8) -> i32 {
        unsafe {
            let osiris_module = LoadLibraryW(w!("Osiris.dll")).unwrap();
            let osiris_ctor_proc =
                GetProcAddress(osiris_module, s!("??0COsiris@@QEAA@XZ")).unwrap() as _;

            OSIRIS_GLOBALS = Some(find_osiris_globals(osiris_ctor_proc).unwrap());

            HOOKS.original.Call.set(*(b as *const *const ()).add(1));
            HOOKS.original.Query.set(*(b as *const *const ()).add(2));

            original::RegisterDivFunctions(a, b)
        }
    }
    fn Call(handle: u32, params: *const OsiArgumentDesc) -> bool {
        original::Call(handle, params)
    }
    fn Query(handle: u32, params: *const OsiArgumentDesc) -> bool {
        original::Query(handle, params)
    }
}

#[derive(Debug, Default)]
struct HookableFunction<T> {
    ptr: Option<T>,
}

impl<T> AsRef<T> for HookableFunction<T> {
    fn as_ref(&self) -> &T {
        match &self.ptr {
            None => panic!("function not initialized"),
            Some(ptr) => ptr,
        }
    }
}

impl<T> AsMut<T> for HookableFunction<T> {
    fn as_mut(&mut self) -> &mut T {
        match &mut self.ptr {
            None => panic!("function not initialized"),
            Some(ptr) => ptr,
        }
    }
}

impl<T> HookableFunction<T> {
    pub const fn new() -> Self {
        Self { ptr: None }
    }

    pub fn set(&mut self, ptr: *const ()) {
        self.ptr = Some(unsafe { mem::transmute_copy(&ptr) });
    }
}

#[link(name = "detours", kind = "static")]
extern "C" {
    fn DetourTransactionBegin();
    fn DetourUpdateThread(handle: HANDLE);
    fn DetourAttach(
        ppPointer: *mut *const libc::c_void,
        pDetour: *const libc::c_void,
    ) -> libc::c_long;
    fn DetourTransactionCommit();
}

fn main() {
    let use_tcp = true;
    if use_tcp {
        StdIo::tcp("127.0.0.1:9003");
    } else {
        StdIo::normal();
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

    unsafe {
        let osiris_module = LoadLibraryW(w!("Osiris.dll")).unwrap();
        let tgt = GetProcAddress(
            osiris_module,
            s!("?RegisterDIVFunctions@COsiris@@QEAAXPEAUTOsirisInitFunction@@@Z"),
        )
        .unwrap();
        HOOKS.original.RegisterDivFunctions.set(tgt as _);
        DetourTransactionBegin();
        DetourUpdateThread(GetCurrentThread());
        DetourAttach(
            HOOKS.original.RegisterDivFunctions.as_mut() as *mut _ as _,
            hooks::RegisterDivFunctions as _,
        );
        DetourTransactionCommit();
    }

    std::thread::spawn(|| {
        let mut buf = String::new();
        loop {
            _print!(">> ");
            let _ = StdIo::get().read_line(&mut buf);
            let func = buf.trim();
            info!("searching for '{func}'");

            if let Some(globals) = unsafe { OSIRIS_GLOBALS } {
                let name = OsiString::from_bytes(b"GetHostCharacter/1");
                let hash = function_name_hash(b"GetHostCharacter") + 1;

                let res = unsafe { (**globals.functions).find(hash, name) };
                info!("{res:?}");

                if let Some(res) = res {
                    let res = unsafe { (**res).handle() };
                    let out =
                        OsiArgumentDesc::from_value(OsiArgumentValue::guid_string(ptr::null()));
                    hooks::Query(res, out);

                    unsafe { _println!("{:?}", (*out).value) };
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
            variables: ptr::null(),
            types: ptr::null(),
            enums: ptr::null(),
            functions: ptr::null(),
            objects: ptr::null(),
            goals: ptr::null(),
            adapters: ptr::null(),
            databases: ptr::null(),
            nodes: ptr::null(),
        }
    }
}

fn print_bytes(buf: &[u8], width: usize) {
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

unsafe fn find_osiris_globals(ctor_proc: *const u8) -> Option<OsirisStaticGlobals> {
    let addr = resolve_real_function_address(ctor_proc);

    let mut globals = [ptr::null::<*const u8>(); 9];
    let mut found_globals = 0;

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
