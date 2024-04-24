use std::{
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr,
};

mod array;
pub(crate) mod glm;
mod map;
mod string;

pub(crate) use array::*;
pub(crate) use map::*;
pub(crate) use string::*;

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
    pub value: T,
    pub is_overriden: bool,
}

impl<T> Deref for OverrideableProperty<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
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

impl<T> Index<usize> for GamePtr<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.ptr.add(index) }
    }
}

impl<T> Index<u32> for GamePtr<T> {
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        unsafe { &*self.ptr.add(index as _) }
    }
}

impl<T> IndexMut<usize> for GamePtr<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { &mut *self.ptr.add(index) }
    }
}

impl<T> IndexMut<u32> for GamePtr<T> {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        unsafe { &mut *self.ptr.add(index as _) }
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
