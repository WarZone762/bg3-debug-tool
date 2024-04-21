use std::{marker::PhantomData, ops::Index};

use super::GamePtr;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Array<T> {
    pub buf: GamePtr<T>,
    pub capacity: u32,
    pub size: u32,
}

impl<T> Index<u32> for Array<T> {
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        if index >= self.size {
            panic!(
                "UninitializedStaticArary index out of bounds: the size is {} but the index is \
                 {index}",
                self.size
            )
        }
        &self.buf[index]
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct StaticArray<T> {
    pub buf: GamePtr<T>,
    pub size: u32,
}

impl<T> Index<u32> for StaticArray<T> {
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        if index >= self.size {
            panic!(
                "UninitializedStaticArary index out of bounds: the size is {} but the index is \
                 {index}",
                self.size
            )
        }
        &self.buf[index]
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct UninitializedStaticArray<T> {
    pub buf: GamePtr<T>,
    pub size: u32,
}

impl<T> UninitializedStaticArray<T> {
    pub fn iter(&self) -> ArrayIter<'_, T> {
        self.into_iter()
    }
}

impl<T> Index<u32> for UninitializedStaticArray<T> {
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        if index >= self.size {
            panic!(
                "UninitializedStaticArary index out of bounds: the size is {} but the index is \
                 {index}",
                self.size
            )
        }
        &self.buf[index]
    }
}

impl<'a, T> IntoIterator for &'a UninitializedStaticArray<T> {
    type IntoIter = ArrayIter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        ArrayIter::new(self.buf.ptr, self.size)
    }
}

#[derive(Debug)]
pub(crate) struct ArrayIter<'a, T> {
    ptr: *const T,
    size: u32,
    index: u32,
    marker: PhantomData<&'a T>,
}

impl<T> ArrayIter<'_, T> {
    pub fn new(ptr: *const T, size: u32) -> Self {
        Self { ptr, size, index: 0, marker: PhantomData }
    }
}

impl<'a, T> Iterator for ArrayIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.size {
            return None;
        }
        let v = unsafe { &*self.ptr.add(self.index as _) };
        self.index += 1;
        Some(v)
    }
}
