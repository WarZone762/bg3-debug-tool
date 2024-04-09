#![feature(let_chains)]
// #![allow(dead_code, unused_variables)]

mod binary_mappings;
mod game_definitions;
mod globals;
mod hooks;
mod script_extender;
mod wrappers;

use std::{io::BufRead, panic, thread};

use widestring::u16cstr;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{GetLastError, BOOL, HANDLE, HMODULE},
        System::{LibraryLoader::LoadLibraryW, SystemInformation::GetSystemDirectoryW},
    },
};

use crate::{
    binary_mappings::init_static_symbols,
    game_definitions::{FixedString, GamePtr, LSStringView},
    globals::Globals,
    script_extender::LibraryManager,
    wrappers::osiris::OsiCall,
};

#[no_mangle]
pub extern "system" fn DllMain(_dll: HANDLE, reason: DllCallReason, _reserved: &u32) -> BOOL {
    match reason {
        DllCallReason::DLL_PROCESS_ATTACH => main(),
        DllCallReason::DLL_PROCESS_DETACH => (),
        _ => (),
    }
    true.into()
}

fn main() {
    let use_tcp = true;
    if use_tcp {
        Globals::io_set(Some(globals::Io::tcp("127.0.0.1:9003")));
    } else {
        Globals::io_set(Some(globals::Io::stdio()));
    }

    let old_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        err!("{info}");
        old_panic_hook(info)
    }));

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

    init_static_symbols().unwrap();
    hooks::osiris::init().unwrap();
    hooks::vulkan::init().unwrap();

    info!("Info");
    warn!("Warning");
    err!("Error");

    thread::spawn(console_thread);
}

fn console_thread() {
    let mut buf = String::new();

    let get_fixed_string = |mut fs: FixedString| -> Option<LSStringView> {
        if fs.index == FixedString::null_index() {
            return None;
        }

        let getter = Globals::static_symbols().ls__FixedString__GetString?;
        let mut sv = LSStringView::new();
        getter(GamePtr::new(&mut fs), GamePtr::new(&mut sv));
        Some(sv)
    };

    fn exec_cmd<'a>(buf: &'a mut String, name: &str) -> &'a str {
        _print!("{name} >> ");
        buf.clear();

        Globals::io_mut().read_line(buf).unwrap();
        buf.trim()
    }

    loop {
        match exec_cmd(&mut buf, "") {
            "search" | "s" => {
                let input = exec_cmd(&mut buf, "search");

                let template_manager =
                    *Globals::static_symbols().ls__GlobalTemplateManager.unwrap();
                let template_bank = template_manager.global_template_bank();

                let mut count = 0;
                for t in template_bank.templates.iter() {
                    if !t.is_null() {
                        let k = t.key;
                        let v = t.value;

                        let name = v.name.as_str();
                        if name.contains(input) {
                            info!("{name:?}: {k:?}, {:?}", get_fixed_string(v.id));
                            count += 1;
                        }
                    }
                }
                info!("{count} entries found");
            }
            "call" | "c" => {
                let input = exec_cmd(&mut buf, "call");

                let call = match syn::parse_str::<OsiCall>(input) {
                    Ok(x) => x,
                    Err(x) => {
                        warn!("{x}");
                        continue;
                    }
                };

                match call.call() {
                    Ok(x) => info!("{x:?}"),
                    Err(x) => warn!("{x}"),
                }
            }
            "query" | "q" => {
                let input = exec_cmd(&mut buf, "query");

                let query = match syn::parse_str::<OsiCall>(input) {
                    Ok(x) => x,
                    Err(x) => {
                        warn!("{x}");
                        continue;
                    }
                };

                match query.query() {
                    Ok(x) => info!("{x:?}"),
                    Err(x) => warn!("{x}"),
                }
            }
            c => warn!("unknown command '{c}'"),
        }
    }
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

#[allow(non_camel_case_types)]
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum DllCallReason {
    DLL_PROCESS_ATTACH = 1,
    DLL_PROCESS_DETACH = 0,
    DLL_THREAD_ATTACH = 2,
    DLL_THREAD_DETACH = 3,
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
