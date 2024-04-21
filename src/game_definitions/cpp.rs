use std::{
    alloc,
    ffi::CStr,
    fmt::{Debug, Display},
};

use super::PtrOrBuf;

#[repr(C)]
pub(crate) struct STDStringOwned {
    pub string: STDString,
}

impl STDStringOwned {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 16 {
            let mut buf = [0u8; 16];
            buf[..bytes.len()].clone_from_slice(bytes);
            Self {
                string: STDString {
                    ptr_or_buf: PtrOrBuf { buf },
                    size: bytes.len() as _,
                    capacity: 15,
                },
            }
        } else {
            let ptr = unsafe {
                let layout = alloc::Layout::from_size_align_unchecked(bytes.len() + 1, 1);
                let ptr = alloc::alloc(layout);
                if ptr.is_null() {
                    alloc::handle_alloc_error(layout);
                }
                ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
                ptr.add(bytes.len()).write(0);
                ptr
            };

            Self {
                string: STDString {
                    ptr_or_buf: PtrOrBuf { ptr },
                    size: bytes.len() as _,
                    capacity: bytes.len() as _,
                },
            }
        }
    }
}

impl Drop for STDStringOwned {
    fn drop(&mut self) {
        if self.string.is_large_mode() {
            unsafe {
                alloc::dealloc(
                    self.string.ptr_or_buf.ptr,
                    alloc::Layout::from_size_align_unchecked((self.string.capacity + 1) as _, 1),
                );
            }
        }
    }
}

#[repr(C)]
pub(crate) struct STDString {
    ptr_or_buf: PtrOrBuf,
    size: u32,
    capacity: u32,
}

impl Debug for STDString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("STDString")
            .field("ptr_or_buf", &self.c_str())
            .field("size", &self.size)
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl Display for STDString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<STDString> for String {
    fn from(value: STDString) -> Self {
        value.to_string()
    }
}

impl From<&STDString> for String {
    fn from(value: &STDString) -> Self {
        value.to_string()
    }
}

impl PartialEq for STDString {
    fn eq(&self, other: &Self) -> bool {
        self.c_str().eq(other.c_str())
    }
}

impl PartialOrd for STDString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.c_str().partial_cmp(other.c_str())
    }
}

impl STDString {
    pub fn c_str(&self) -> &CStr {
        if self.is_large_mode() {
            unsafe { CStr::from_ptr(self.ptr_or_buf.ptr as _) }
        } else {
            unsafe { CStr::from_ptr(self.ptr_or_buf.buf.as_ptr() as _) }
        }
    }

    pub fn as_str(&self) -> &str {
        if self.is_large_mode() {
            unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    self.ptr_or_buf.ptr,
                    self.size as _,
                ))
            }
        } else {
            unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    self.ptr_or_buf.buf.as_ptr(),
                    self.size as _,
                ))
            }
        }
    }

    fn is_large_mode(&self) -> bool {
        self.capacity > 15
    }
}
