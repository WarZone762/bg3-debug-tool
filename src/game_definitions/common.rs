use std::{
    alloc,
    ffi::CStr,
    fmt::Debug,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr,
};

use crate::globals::Globals;

pub(crate) mod glm {
    #[derive(Debug)]
    #[repr(C)]
    pub(crate) struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(Debug)]
    #[repr(C)]
    pub(crate) struct Quat {
        w: f32,
        x: f32,
        y: f32,
        z: f32,
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Transform {
    rotation_quat: glm::Quat,
    translate: glm::Vec3,
    scale: glm::Vec3,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct OverrideableProperty<T> {
    value: T,
    is_overriden: bool,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct FixedString {
    pub index: u32,
}

impl FixedString {
    pub fn new() -> Self {
        Self { index: Self::null_index() }
    }

    pub fn get(&self) -> Option<LSStringView> {
        if self.index == FixedString::null_index() {
            return None;
        }

        let getter = Globals::static_symbols().ls__FixedString__GetString?;
        let mut sv = LSStringView::new();
        getter(self.into(), GamePtr::new(&mut sv));
        Some(sv)
    }

    pub fn as_str(&self) -> &str {
        self.get().map(|x| x.as_str()).unwrap_or("")
    }

    pub const fn null_index() -> u32 {
        0xFFFFFFFF
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FixedStringHeader {
    hash: u32,
    ref_count: u32,
    length: u32,
    id: u32,
    next_free_index: u64,
}

#[repr(C)]
pub(crate) struct LSStringView<'a> {
    data: *const u8,
    size: i32,
    marker: PhantomData<&'a str>,
}

impl Debug for LSStringView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LSStringView")
            .field("data", &self.as_str())
            .field("size", &self.size)
            .finish()
    }
}

impl<'a> LSStringView<'a> {
    pub fn new() -> Self {
        Self { data: std::ptr::null(), size: 0, marker: PhantomData::default() }
    }

    pub fn as_str(&self) -> &'a str {
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.data, self.size as _))
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Array<T> {
    buf: GamePtr<T>,
    capacity: u32,
    size: u32,
}

pub(crate) type Map<TKey, TValue> = MapInternals<TKey, TValue>;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct MapInternals<TKey, TValue> {
    hash_size: u32,
    hash_table: GamePtr<GamePtr<MapNode<TKey, TValue>>>,
    item_count: u32,
}

impl<K: 'static, V: 'static> MapInternals<K, V> {
    pub fn iter(&self) -> impl Iterator<Item = GamePtr<MapNode<K, V>>> {
        MapIter::new(self)
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct MapNode<TKey, TValue> {
    pub next: GamePtr<MapNode<TKey, TValue>>,
    pub key: TKey,
    pub value: TValue,
}

#[derive(Debug)]
pub(crate) struct MapIter<'a, K, V> {
    iter: std::iter::Filter<
        std::slice::Iter<'a, GamePtr<MapNode<K, V>>>,
        fn(&&GamePtr<MapNode<K, V>>) -> bool,
    >,
    elem: GamePtr<MapNode<K, V>>,
}

impl<K, V> MapIter<'_, K, V> {
    pub fn new(map: &MapInternals<K, V>) -> Self {
        let arr = unsafe {
            std::slice::from_raw_parts::<GamePtr<MapNode<K, V>>>(
                map.hash_table.ptr,
                map.hash_size as _,
            )
        };
        Self { iter: arr.iter().filter(|e| !e.is_null()), elem: GamePtr::null() }
    }
}

impl<K, V> Iterator for MapIter<'_, K, V> {
    type Item = GamePtr<MapNode<K, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.elem.is_null() {
            self.elem = *self.iter.next()?;
        }

        let elem = self.elem;

        self.elem = self.elem.next;

        Some(elem)
    }
}

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

#[repr(C)]
pub(crate) union PtrOrBuf {
    pub ptr: *mut u8,
    pub buf: [u8; 16],
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct GamePtr<T> {
    pub ptr: *mut T,
}

impl<T> Deref for GamePtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for GamePtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<T> PartialEq for GamePtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T> Eq for GamePtr<T> {}

impl<T> PartialOrd for GamePtr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for GamePtr<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ptr.cmp(&other.ptr)
    }
}

impl<T> Clone for GamePtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for GamePtr<T> {}

impl<T> From<*const T> for GamePtr<T> {
    fn from(value: *const T) -> Self {
        Self::new(value as *mut T)
    }
}

impl<T> From<*mut T> for GamePtr<T> {
    fn from(value: *mut T) -> Self {
        Self::new(value)
    }
}

impl<T> From<&T> for GamePtr<T> {
    fn from(value: &T) -> Self {
        Self::new(value as *const T as *mut T)
    }
}

impl<T> From<&mut T> for GamePtr<T> {
    fn from(value: &mut T) -> Self {
        Self::new(value)
    }
}

impl<T> GamePtr<T> {
    pub fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }

    pub const fn null() -> Self {
        Self { ptr: ptr::null_mut() }
    }

    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    pub fn as_usize(&self) -> usize {
        self.ptr as usize
    }

    pub fn as_ref<'a>(self) -> &'a T {
        unsafe { &*self.ptr }
    }
}
