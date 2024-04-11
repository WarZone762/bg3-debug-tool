use std::mem;

use ash::vk::DWORD;
use windows::{
    core::{PCSTR, PSTR},
    Win32::{
        Foundation::{BOOL, HANDLE},
        Security::SECURITY_ATTRIBUTES,
        System::Threading::{
            CreateProcessA, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOA,
        },
    },
};

pub(crate) mod osiris;
pub(crate) mod vulkan;

#[macro_export]
macro_rules! hook_definitions {
    {
        $mod_name:ident($dll_name:literal) {
            $(
                $(#[symbol_name = $symbol_name:literal])?
                $(#[no_init = $init:ident])?
                fn $name:ident($($arg_name:ident: $arg:ty),* $(,)?) $(-> $ret: ty)? $body: block
            )*
        }
    } => {
        pub(crate) fn init() -> anyhow::Result<()> {
            unsafe {
                let $mod_name = windows::Win32::System::LibraryLoader::LoadLibraryW(
                    windows::core::w!($dll_name)
                )?;
                $crate::hooks::DetourTransactionBegin();
                $crate::hooks::DetourUpdateThread(
                    windows::Win32::System::Threading::GetCurrentThread()
                );

                $(
                    $crate::if_no_init_meta!(
                        $crate::init_hook_from_name!(
                            $mod_name,
                            $name $(, $symbol_name)?
                        ) $(, $init)?
                    );
                )*

                $crate::hooks::DetourTransactionCommit();
            }

            Ok(())
        }

        #[allow(non_snake_case, dead_code)] mod _hooks {
            use super::*;
            $(
                pub extern "C" fn $name($($arg_name: $arg),*) $(-> $ret)? $body
            )*
        }

        pub(crate) use _hooks::*;

        #[allow(non_snake_case)]
        #[derive(Debug, Default)]
        pub(crate) struct Hooks {
            $(
                pub $name: $crate::hooks::HookableFunction<extern "C" fn($($arg_name: $arg),*) $(-> $ret)?>,
            )*
        }

        impl Hooks {
            pub const fn new() -> Self {
                Self {
                    $(
                        $name: $crate::hooks::HookableFunction::new(),
                    )*
                }
            }
        }

        static mut HOOKS: Hooks = Hooks::new();

        #[allow(non_snake_case, dead_code)]
        mod original {
            use super::*;
            $(
                pub extern "C" fn $name($($arg_name: $arg),*) $(-> $ret)? {
                    unsafe { HOOKS.$name.as_ref()($($arg_name),*) }
                }
            )*
        }
    };
}

#[macro_export]
macro_rules! if_no_init_meta {
    ($expr:expr,yes) => {};
    ($expr:expr $(,)?) => {
        $expr
    };
}

#[macro_export]
macro_rules! init_hook {
    ($name:ident, $tgt:expr) => {
        HOOKS.$name.set($tgt as _);
        $crate::hooks::DetourAttach(HOOKS.$name.as_mut() as *mut _ as _, $name as _)
    };
}

#[macro_export]
macro_rules! init_hook_from_name {
    ($module:expr, $name:ident) => {
        $crate::init_hook_from_name!($module, $name, stringify!($name))
    };
    ($module:expr, $name:ident, $symbol_name:expr) => {{
        let Some(tgt) = windows::Win32::System::LibraryLoader::GetProcAddress(
            $module,
            windows::core::PCSTR(concat!($symbol_name, "\0").as_ptr()),
        ) else {
            anyhow::bail!(concat!("Failed to find ", $symbol_name));
        };
        $crate::init_hook!($name, tgt);
    }};
}

#[link(name = "detours", kind = "static")]
extern "system" {
    fn DetourTransactionBegin();
    fn DetourUpdateThread(handle: HANDLE);
    fn DetourAttach(
        ppPointer: *mut *const libc::c_void,
        pDetour: *const libc::c_void,
    ) -> libc::c_long;
    fn DetourTransactionCommit();
}

#[derive(Debug)]
pub(crate) struct HookableFunction<T> {
    ptr: Option<T>,
}

impl<T> Default for HookableFunction<T> {
    fn default() -> Self {
        Self { ptr: Default::default() }
    }
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
