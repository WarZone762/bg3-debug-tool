use std::fmt::Display;

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{GetLastError, LocalFree},
        Storage::FileSystem::{VerQueryValueW, VS_FIXEDFILEINFO, VS_VERSION_INFO},
        System::{
            LibraryLoader::{
                FindResourceW, FreeResource, GetModuleHandleW, LoadResource, LockResource,
                SizeofResource,
            },
            Memory::{LocalAlloc, LMEM_FIXED},
        },
        UI::WindowsAndMessaging::RT_VERSION,
    },
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct GameVersionInfo {
    pub(crate) major: u16,
    pub(crate) minor: u16,
    pub(crate) revision: u16,
    pub(crate) build: u16,
}

impl Display for GameVersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}.{}.{}.{}", self.major, self.minor, self.revision, self.build)
    }
}

impl GameVersionInfo {
    #[inline]
    pub(crate) fn is_supported(&self) -> bool {
        self.major == 4 && self.minor >= 37
    }
}

pub(crate) struct ScriptExtender {
    libraries: LibraryManager,
}

pub(crate) struct LibraryManager {}

impl LibraryManager {
    pub(crate) fn game_version() -> windows::core::Result<GameVersionInfo> {
        unsafe {
            let game_module = GetModuleHandleW(w!("bg3.exe"))
                .or_else(|_| GetModuleHandleW(w!("bg3_dx11.exe")))?;

            let resource = FindResourceW(game_module, PCWSTR(VS_VERSION_INFO as _), RT_VERSION);
            if resource.is_invalid() {
                return Err(GetLastError().into());
            }
            let size = SizeofResource(game_module, resource) as usize;
            let data = LoadResource(game_module, resource)?;
            let resource = LockResource(data);
            if resource.is_null() {
                return Err(GetLastError().into());
            }

            let resource_copy = LocalAlloc(LMEM_FIXED, size)?;
            resource_copy.0.copy_from_nonoverlapping(resource, size);

            let mut ver_len = 0;
            let mut fixed_file_info: *mut VS_FIXEDFILEINFO = std::ptr::null_mut();
            if !VerQueryValueW(
                resource_copy.0,
                w!("\\"),
                &mut fixed_file_info as *mut _ as _,
                &mut ver_len,
            )
            .as_bool()
            {
                return Err(GetLastError().into());
            }

            let version = GameVersionInfo {
                major: ((*fixed_file_info).dwFileVersionMS >> 16) as _,
                minor: ((*fixed_file_info).dwFileVersionMS & 0xFFFF) as _,
                revision: ((*fixed_file_info).dwFileVersionLS >> 16) as _,
                build: ((*fixed_file_info).dwFileVersionLS & 0xFFFF) as _,
            };

            let _ = LocalFree(resource_copy);
            FreeResource(data);

            Ok(version)
        }
    }
}
