#![feature(let_chains)]
// #![allow(dead_code, unused_variables)]

mod binary_mappings;
mod game_definitions;
mod globals;
mod hooks;
mod menu;
mod script_extender;
mod wrappers;

use std::{io::BufRead, panic, thread};

use hudhook::{hooks::dx11::ImguiDx11Hooks, Hudhook};
use windows::{
    core::w,
    Win32::{
        Foundation::{BOOL, HMODULE},
        System::{
            LibraryLoader::GetModuleHandleW,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
};

use crate::{
    binary_mappings::init_static_symbols, globals::Globals, script_extender::LibraryManager,
    wrappers::osiris::OsiCall,
};

#[no_mangle]
pub extern "system" fn DllMain(_dll: HMODULE, reason: u32, _reserved: &u32) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => main(),
        DLL_PROCESS_DETACH => (),
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

    if let Ok(version) = LibraryManager::game_version() {
        if version.is_supported() {
            info!("Game version {version} OK");
        } else {
            err!("Game versino {version} is not supported, please upgrade!");
            panic!(
                "Scrip Extender doesn't support game versions below v4.37,
    please upgrade!"
            );
        }
    } else {
        err!("Failed to retrieve game version info.");
    }

    init_static_symbols().unwrap();
    hooks::osiris::hook().unwrap();

    let menu = menu::Menu::new();
    let is_dx11 = unsafe { GetModuleHandleW(w!("bg3_dx11.exe")) }.is_ok_and(|x| !x.is_invalid());
    if is_dx11 {
        std::thread::spawn(move || {
            if let Err(e) = Hudhook::builder().with::<ImguiDx11Hooks>(menu).build().apply() {
                err!("Couldn't apply hooks: {e:?}");
            }
        });
    } else {
        hooks::vulkan::init(menu).unwrap();
    }

    thread::spawn(console_thread);
}

fn console_thread() {
    let mut buf = String::new();

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
                            info!("{name:?}: {k:?}, {}", v.id.as_str());
                            info!("{}", v.get_type().as_str());
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
