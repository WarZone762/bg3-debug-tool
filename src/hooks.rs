use std::{ffi::CStr, mem};

use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{HANDLE, HMODULE},
        System::LibraryLoader::GetProcAddress,
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
        pub(crate) fn hook() -> anyhow::Result<()> {
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
        #[derive(Debug)]
        pub(crate) struct Hooks {
            $(
                pub $name: $crate::hooks::HookableFunction<extern "C" fn($($arg_name: $arg),*) $(-> $ret)?>,
            )*
        }

        impl Hooks {
            pub const fn new() -> Self {
                Self {
                    $(
                        $name: $crate::hooks::HookableFunction::new($name),
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
                    unsafe { HOOKS.$name.original()($($arg_name),*) }
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
macro_rules! init_hook_from_name {
    ($module:expr, $name:ident) => {
        $crate::init_hook_from_name!($module, $name, stringify!($name))
    };
    ($module:expr, $name:ident, $symbol_name:expr) => {{
        HOOKS.$name.find_attach(
            $module,
            std::ffi::CStr::from_ptr(concat!($symbol_name, "\0").as_ptr() as _),
        );
    }};
}

#[macro_export]
macro_rules! fn_definitions {
    {
        $mod_name:ident($dll_name:literal) {
            $(
                $(#[symbol_name = $symbol_name:literal])?
                $(#[no_init = $init:ident])?
                fn $name:ident($($arg_name:ident: $arg:ty),* $(,)?) $(-> $ret: ty)?;
            )*
        }
    } => {
        // pub(crate) fn init_functions() -> anyhow::Result<()> {
        //     unsafe {
        //         let $mod_name = windows::Win32::System::LibraryLoader::LoadLibraryW(
        //             windows::core::w!($dll_name)
        //         )?;
        //
        //         $(
        //             $crate::if_no_init_meta!(
        //                 $crate::init_hook_from_name!(
        //                     $mod_name,
        //                     $name $(, $symbol_name)?
        //                 ) $(, $init)?
        //             );
        //         )*
        //     }
        //
        //     Ok(())
        // }

        #[allow(non_snake_case)]
        #[derive(Debug)]
        pub(crate) struct Functions {
            $(
                pub $name: $crate::hooks::GameFunction<extern "C" fn($($arg_name: $arg),*) $(-> $ret)?>,
            )*
        }

        impl Functions {
            pub const fn new() -> Self {
                Self {
                    $(
                        $name: $crate::hooks::GameFunction::new(),
                    )*
                }
            }
        }

        static mut FUNCS: Functions = Functions::new();

        $(
            #[allow(non_snake_case)]
            pub extern "C" fn $name($($arg_name: $arg),*) $(-> $ret)? {
                unsafe { FUNCS.$name.get()($($arg_name),*) }
            }
        )*
    };
}

#[link(name = "detours", kind = "static")]
extern "system" {
    pub fn DetourTransactionBegin();
    pub fn DetourUpdateThread(handle: HANDLE);
    pub fn DetourAttach(
        ppPointer: *mut *const libc::c_void,
        pDetour: *const libc::c_void,
    ) -> libc::c_long;
    pub fn DetourTransactionCommit();
}

#[derive(Debug)]
pub(crate) struct GameFunction<T>(Option<T>);

impl<T> GameFunction<T> {
    pub const fn new() -> Self {
        Self(None)
    }

    pub fn set(&mut self, ptr: *const ()) {
        self.0 = unsafe { Some(mem::transmute_copy(&ptr)) };
    }

    pub fn get(&self) -> &T {
        match &self.0 {
            None => panic!("function not initialized"),
            Some(ptr) => ptr,
        }
    }
}

#[derive(Debug)]
pub(crate) struct HookableFunction<T> {
    original: Option<T>,
    hook: T,
}

impl<T> HookableFunction<T> {
    pub const fn new(hook: T) -> Self {
        Self { original: None, hook }
    }

    pub fn original(&self) -> &T {
        match &self.original {
            None => panic!("function not initialized"),
            Some(ptr) => ptr,
        }
    }

    /// Must be called after [`DetourTransactionBegin`] and before
    /// [`DetourTransactionCommit`]
    pub fn find_attach(&mut self, module: impl Into<HMODULE>, symbol_name: &CStr) {
        let Some(original) =
            (unsafe { GetProcAddress(module.into(), PCSTR(symbol_name.as_ptr() as _)) })
        else {
            panic!("Failed to find {:?}", symbol_name);
        };
        self.attach(original as _);
    }

    /// Must be called after [`DetourTransactionBegin`] and before
    /// [`DetourTransactionCommit`]
    pub fn attach(&mut self, original: *const ()) -> i32 {
        self.original = Some(unsafe { mem::transmute_copy(&original) });
        unsafe {
            DetourAttach(
                self.original.as_ref().unwrap() as *const _ as _,
                mem::transmute_copy(&self.hook),
            )
        }
    }
}
