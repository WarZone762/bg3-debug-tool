use super::{Array, GamePtr, StaticArray, UninitializedStaticArray};

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
    map_arr: &'a [GamePtr<MapNode<K, V>>],
    elem: GamePtr<MapNode<K, V>>,
    index: u32,
}

impl<K, V> MapIter<'_, K, V> {
    pub fn new(map: &MapInternals<K, V>) -> Self {
        let map_arr = unsafe { std::slice::from_raw_parts(map.hash_table.ptr, map.hash_size as _) };
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
    hash_set: MultiHashSet<TKey>,
    values: UninitializedStaticArray<TValue>,
}

impl<TKey: GameHash + Eq, TValue> MultiHashMap<TKey, TValue> {
    pub fn try_get(&self, key: &TKey) -> Option<&TValue> {
        let index = self.find_index(key)?;
        Some(&self.values[index])
    }

    pub fn find_index(&self, key: &TKey) -> Option<u32> {
        self.hash_set.find_index(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TValue> {
        self.values.iter()
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct MultiHashSet<T: GameHash + Eq> {
    hash_keys: StaticArray<i32>,
    next_ids: Array<i32>,
    keys: Array<T>,
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
