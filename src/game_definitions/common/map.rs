use std::ops::Deref;

use super::{Array, GamePtr, StaticArray, UninitializedStaticArray};

pub(crate) type Map<TKey, TValue> = MapInternals<TKey, TValue>;
pub(crate) type RefMap<TKey, TValue> = RefMapInternals<TKey, TValue>;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct RefMapInternals<TKey, TValue> {
    pub item_count: u32,
    pub hash_size: u32,
    pub hash_table: GamePtr<GamePtr<MapNode<TKey, TValue>>>,
}

impl<K: 'static, V: 'static> RefMapInternals<K, V> {
    pub fn iter(&self) -> impl Iterator<Item = GamePtr<MapNode<K, V>>> {
        MapIter::new(self.hash_table, self.hash_size)
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct MapInternals<TKey, TValue> {
    pub hash_size: u32,
    pub hash_table: GamePtr<GamePtr<MapNode<TKey, TValue>>>,
    pub item_count: u32,
}

impl<K: 'static, V: 'static> MapInternals<K, V> {
    pub fn iter(&self) -> impl Iterator<Item = GamePtr<MapNode<K, V>>> {
        MapIter::new(self.hash_table, self.hash_size)
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
    map_arr: &'a [GamePtr<MapNode<K, V>>],
    elem: GamePtr<MapNode<K, V>>,
    index: u32,
}

impl<K, V> MapIter<'_, K, V> {
    pub fn new(hash_table: GamePtr<GamePtr<MapNode<K, V>>>, hash_size: u32) -> Self {
        let map_arr = unsafe { std::slice::from_raw_parts(hash_table.ptr, hash_size as _) };
        Self { map_arr, elem: GamePtr::null(), index: 0 }
    }
}

impl<K, V> Iterator for MapIter<'_, K, V> {
    type Item = GamePtr<MapNode<K, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.elem.is_null() {
            if self.index as usize == self.map_arr.len() {
                return None;
            }
            self.elem = self.map_arr[self.index as usize];
            self.index += 1;
        }
        let val = self.elem;
        self.elem = val.next;
        Some(val)
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct MultiHashMap<TKey: GameHash + Eq, TValue> {
    pub hash_set: MultiHashSet<TKey>,
    pub values: UninitializedStaticArray<TValue>,
}

impl<TKey: GameHash + Eq, TValue> MultiHashMap<TKey, TValue> {
    pub fn try_get(&self, key: &TKey) -> Option<&TValue> {
        let index = self.find_index(key)?;
        Some(&self.values[index])
    }

    pub fn find_index(&self, key: &TKey) -> Option<u32> {
        self.hash_set.find_index(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&TKey, &TValue)> {
        self.hash_set.keys.iter().zip(self.values.iter())
    }

    pub fn entries(&self) -> impl Iterator<Item = (&TKey, &TValue)> {
        self.hash_set.iter_indecies().map(|x| (&self.hash_set.keys[x], &self.values[x]))
    }
}

impl<K: GameHash + Eq, V> Deref for MultiHashMap<K, V> {
    type Target = MultiHashSet<K>;

    fn deref(&self) -> &Self::Target {
        &self.hash_set
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct MultiHashSet<T: GameHash + Eq> {
    pub hash_keys: StaticArray<i32>,
    pub next_ids: Array<i32>,
    pub keys: Array<T>,
}

impl<T: GameHash + Eq> MultiHashSet<T> {
    pub fn find_index(&self, key: &T) -> Option<u32> {
        if self.hash_keys.size == 0 {
            return None;
        }

        let mut key_index = self.hash_keys[(key.hash() % self.hash_keys.size as u64) as u32];
        while key_index >= 0 {
            if &self.keys[key_index as u32] == key {
                return Some(key_index as u32);
            }
            key_index = self.next_ids[key_index as u32];
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.iter_indecies().map(|x| &self.keys[x])
    }

    pub fn iter_indecies(&self) -> impl Iterator<Item = u32> + '_ {
        self.hash_keys.iter().filter(|x| **x >= 0).map(|x| *x as u32)
    }

    pub fn len(&self) -> u32 {
        self.keys.len() as _
    }
}

pub(crate) trait GameHash {
    fn hash(&self) -> u64;
}

impl GameHash for u8 {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for i8 {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for u16 {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for i16 {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for u32 {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for i32 {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for u64 {
    fn hash(&self) -> u64 {
        *self
    }
}

impl GameHash for i64 {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for usize {
    fn hash(&self) -> u64 {
        *self as u64
    }
}

impl GameHash for isize {
    fn hash(&self) -> u64 {
        *self as u64
    }
}
