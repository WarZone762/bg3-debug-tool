use std::sync::atomic;

use anyhow::anyhow;
use ash::vk::DWORD;

use super::{Array, FixedString, GameHash, GamePtr, LSStringView, MultiHashMap, STDString};
use crate::globals::Globals;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TranslatedStringRepository<'a> {
    field_0: i32,
    translated_strings: [GamePtr<TextPool<'a>>; 9],
    fallback_pool: GamePtr<TextPool<'a>>,
    versioned_fallback_pool: GamePtr<TextPool<'a>>,
    field_60: Array<*const ()>,
    argument_strings: MultiHashMap<FixedString, TranslatedArgumentStringBuffer>,
    text_to_string_key: MultiHashMap<FixedString, RuntimeStringHandle>,
    lock: ThreadedFastLock,
    is_loaded: bool,
}

impl<'a> TranslatedStringRepository<'a> {
    pub fn translated_string(&self, handle: &RuntimeStringHandle) -> Option<LSStringView<'a>> {
        self.translated_strings[0]
            .texts
            .try_get(handle)
            .or_else(|| self.versioned_fallback_pool.texts.try_get(handle))
            .or_else(|| self.fallback_pool.texts.try_get(handle))
            .copied()
    }
}

pub(crate) fn translated_string(handle: FixedString) -> Option<LSStringView<'static>> {
    let repo = Globals::static_symbols().ls__gTranslatedStringRepository?;
    repo.translated_string(&RuntimeStringHandle { handle, version: 0 })
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TextPool<'a> {
    strings: Array<GamePtr<STDString>>,
    texts: MultiHashMap<RuntimeStringHandle, LSStringView<'a>>,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TextEntry<'a> {
    handle: GamePtr<RuntimeStringHandle>,
    text: GamePtr<LSStringView<'a>>,
    field_10: i64,
    field_18: i64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct TranslatedString {
    pub handle: RuntimeStringHandle,
    pub argument_string: RuntimeStringHandle,
}

impl TryInto<String> for TranslatedString {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        self.get().map(|x| x.into()).ok_or(anyhow!("failed to find TranslatedString"))
    }
}

impl TranslatedString {
    pub fn get(&self) -> Option<LSStringView> {
        let repo = Globals::static_symbols().ls__gTranslatedStringRepository?;
        repo.translated_string(&self.handle)
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TranslatedArgumentStringBuffer {
    arguments_buffer_2: GamePtr<u8>,
    needs_formatting: bool,
    argument_buffer: GamePtr<u8>,
    formatted: STDString,
    arguments_length: u32,
    _pad: u32,
    handle: RuntimeStringHandle,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct RuntimeStringHandle {
    handle: FixedString,
    version: u16,
}

impl PartialEq for RuntimeStringHandle {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl Eq for RuntimeStringHandle {}

impl GameHash for RuntimeStringHandle {
    fn hash(&self) -> u64 {
        self.handle.hash()
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ThreadedFastLock {
    fast_lock: atomic::AtomicU32,
    current_therad_id: DWORD,
    enter_count: DWORD,
}
