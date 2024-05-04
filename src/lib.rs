#![feature(
    let_chains,
    unboxed_closures,
    fn_traits,
    min_specialization,
    debug_closure_helpers,
    core_intrinsics
)]
#![allow(clippy::missing_transmute_annotations)]

mod binary_mappings;
mod game_definitions;
mod globals;
mod hooks;
mod menu;
mod version;
mod wrappers;

use std::panic;

use widestring::{u16cstr, U16CStr};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{BOOL, HANDLE, HMODULE},
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE,
        },
        System::{
            LibraryLoader::GetModuleHandleW,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
            IO::OVERLAPPED,
        },
    },
};

use crate::{binary_mappings::init_static_symbols, globals::Globals};

// static SAMPLE_HANDLE: std::sync::Mutex<Option<HANDLE>> =
// std::sync::Mutex::new(None);
//
// hook_definitions! {
// kernel("kernel32.dll") {
//     fn CreateFileW(
//         lp_file_name: PCWSTR,
//         dw_desired_access: u32,
//         dw_share_mode: FILE_SHARE_MODE,
//         lp_security_attributes: *const SECURITY_ATTRIBUTES,
//         dw_creation_disposition: FILE_CREATION_DISPOSITION,
//         dw_flags_and_attributes: FILE_FLAGS_AND_ATTRIBUTES,
//         h_template_file: HANDLE,
//     ) -> HANDLE {
//         let handle = original::CreateFileW(
//             lp_file_name,
//             dw_desired_access,
//             dw_share_mode,
//             lp_security_attributes,
//             dw_creation_disposition,
//             dw_flags_and_attributes,
//             h_template_file,
//         );
//
//         unsafe {
//             if U16CStr::from_ptr_str(lp_file_name.0)
//                 .as_slice()
//
// .ends_with(u16cstr!("SampleEquipmentMod_Icons.dds").as_slice())             {
//                 let mut hit = SAMPLE_HANDLE.lock().unwrap();
//                 *hit = Some(handle);
//             }
//         }
//
//         handle
//
//     }
//
//     fn ReadFile(
//         h_file: HANDLE,
//         lp_buffer: *mut u8,
//         n_number_of_bytes_to_read: u32,
//         lp_number_of_bytes_read: *mut u32,
//         lp_overlapped: *mut OVERLAPPED,
//     ) -> BOOL {
//         let res = original::ReadFile(
//             h_file,
//             lp_buffer,
//             n_number_of_bytes_to_read,
//             lp_number_of_bytes_read,
//             lp_overlapped
//         );
//
//         let handle = SAMPLE_HANDLE.lock().unwrap();
//         if handle.is_some_and(|x| x == h_file) {
//             info!("{lp_buffer:?}");
//             println!("Reading SampleEquipmentMod_Icons\0");
//             // info!("reading");
//             // unsafe {
//             //     std::intrinsics::breakpoint();
//             // }
//         }
//         drop(handle);
//
//         res
//     }
// }
// }

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

    unsafe { libc::atexit(atexit_handler) };

    if let Ok(version) = version::game_version() {
        if version.is_supported() {
            info!("Game version {version} OK");
        } else {
            panic!("Game versino {version} is not supported, please upgrade!");
        }
    } else {
        err!("Failed to retrieve game version info.");
    }

    // hook().unwrap();
    if let Err(x) = init() {
        panic!("{x}");
    }
}

fn init() -> anyhow::Result<()> {
    init_static_symbols()?;
    hooks::osiris::init()?;

    let menu = menu::Menu::new();
    let menu_new = menu::egui_vulkan::Menu::default();
    let is_dx11 = unsafe { GetModuleHandleW(w!("bg3_dx11.exe")) }.is_ok_and(|x| !x.is_invalid());
    if is_dx11 {
        hooks::dx11::init(menu)?;
    } else {
        // hooks::vulkan::init(menu)?;
        hooks::vulkan_egui::init(menu_new)?;
    }

    Ok(())
}

extern "C" fn atexit_handler() {
    if std::path::Path::new("steam_appid.txt").try_exists().is_ok_and(|x| x) {
        std::fs::remove_file("steam_appid.txt").expect("failed to remove 'steam_appid.txt");
    }
}
