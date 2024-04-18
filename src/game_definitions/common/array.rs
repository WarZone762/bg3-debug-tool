use std::ops::Index;

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
