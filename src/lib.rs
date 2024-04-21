#![feature(let_chains, unboxed_closures, fn_traits)]

mod binary_mappings;
mod game_definitions;
mod globals;
mod hooks;
mod menu;
mod version;
mod wrappers;

use std::panic;

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

use crate::{binary_mappings::init_static_symbols, globals::Globals};

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
    if let Some(port) =
        std::env::var("BG3_DEBUG_TOOL_PORT").ok().and_then(|x| x.parse::<u16>().ok())
    {
        Globals::io_set(Some(globals::Io::tcp(format!("127.0.0.1:{port}"))));
    } else {
        Globals::io_set(Some(globals::Io::stdio()));
    }

    let old_panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        err!("{info}");
        old_panic_hook(info)
    }));

    if let Ok(version) = version::game_version() {
        if version.is_supported() {
            info!("Game version {version} OK");
        } else {
            panic!("Game versino {version} is not supported, please upgrade!");
        }
    } else {
        err!("Failed to retrieve game version info.");
    }

    if let Err(x) = init() {
        panic!("{x}");
    }
}

fn init() -> anyhow::Result<()> {
    init_static_symbols()?;
    hooks::osiris::init()?;

    let menu = menu::Menu::new();
    let is_dx11 = unsafe { GetModuleHandleW(w!("bg3_dx11.exe")) }.is_ok_and(|x| !x.is_invalid());
    if is_dx11 {
        std::thread::spawn(move || {
            if let Err(e) = Hudhook::builder().with::<ImguiDx11Hooks>(menu).build().apply() {
                panic!("Couldn't apply hooks: {e:?}");
            }
        });
    } else {
        hooks::vulkan::init(menu)?;
    }
    Ok(())
}
