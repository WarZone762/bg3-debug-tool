#![windows_subsystem = "windows"]

use std::{mem, ptr};

use ash::vk::SECURITY_ATTRIBUTES;
use windows::{
    core::{s, PCSTR, PSTR},
    Win32::{
        Foundation::BOOL,
        System::Threading::{PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOA},
    },
};

fn main() {
    let startup_info =
        STARTUPINFOA { cb: mem::size_of::<STARTUPINFOA>() as _, ..Default::default() };
    let mut proc_info = PROCESS_INFORMATION::default();
    unsafe {
        DetourCreateProcessWithDllExA(
            // s!("bg3_dx11.exe"),
            s!("bg3.exe"),
            PSTR::null(),
            ptr::null(),
            ptr::null(),
            true.into(),
            PROCESS_CREATION_FLAGS::default(),
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
