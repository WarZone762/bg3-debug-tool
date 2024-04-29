#![windows_subsystem = "windows"]

use std::{fs::File, io::Write, mem, ptr};

use clap::Parser;
use windows::{
    core::{s, PCSTR, PSTR},
    Win32::{
        Foundation::BOOL,
        Security::SECURITY_ATTRIBUTES,
        System::Threading::{PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOA},
    },
};

#[derive(Debug, Parser)]
struct Args {
    /// Launch the DirectX 11 version of the game
    #[arg(long)]
    dx11: bool,
    /// TCP port to use for IO
    #[arg(long)]
    port: Option<u16>,
}

fn main() {
    let args = Args::parse();
    let exe = if args.dx11 { s!("bg3_dx11.exe") } else { s!("bg3.exe") };
    if let Some(port) = args.port {
        std::env::set_var("BG3_DEBUG_TOOL_PORT", port.to_string());
    }

    File::create("steam_appid.txt")
        .expect("failed to create 'steam_appid.txt'")
        .write_all(b"1086940\n")
        .expect("failed to write to 'steam_appid.txt'");

    let startup_info =
        STARTUPINFOA { cb: mem::size_of::<STARTUPINFOA>() as _, ..Default::default() };
    let mut proc_info = PROCESS_INFORMATION::default();

    unsafe {
        DetourCreateProcessWithDllExA(
            exe,
            PSTR::null(),
            ptr::null(),
            ptr::null(),
            true.into(),
            PROCESS_CREATION_FLAGS(0),
            ptr::null(),
            PCSTR::null(),
            &startup_info,
            &mut proc_info,
            s!("bg3_debug_tool.dll"),
            None,
        );
    };
}
#[link(name = "detours", kind = "static")]
extern "system" {
    fn DetourCreateProcessWithDllExA(
        lpApplicationName: PCSTR,
        lpCommandLine: PSTR,
        lpProcessAttributes: *const SECURITY_ATTRIBUTES,
        lpThreadAttributes: *const SECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: PROCESS_CREATION_FLAGS,
        lpEnvironment: *const libc::c_void,
        lpCurrentDirectory: PCSTR,
        lpStartupInfo: *const STARTUPINFOA,
        lpProcessInformation: *mut PROCESS_INFORMATION,
        lpDllName: PCSTR,
        pfCreateProcessW: Option<PDETOUR_CREATE_PROCESS_ROUTINEA>,
    ) -> BOOL;
}

#[allow(non_camel_case_types)]
type PDETOUR_CREATE_PROCESS_ROUTINEA = extern "system" fn(
    lpApplicationName: PCSTR,
    lpCommandLine: PSTR,
    lpProcessAttributes: *const SECURITY_ATTRIBUTES,
    lpThreadAttributes: *const SECURITY_ATTRIBUTES,
    bInheritHandles: BOOL,
    dwCreationFlags: PROCESS_CREATION_FLAGS,
    lpEnvironment: *const libc::c_void,
    lpCurrentDirectory: PCSTR,
    lpStartupInfo: *const STARTUPINFOA,
    lpProcessInformation: *mut PROCESS_INFORMATION,
) -> BOOL;
