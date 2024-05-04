use super::{FixedString, GameHash, GamePtr, Map, RefMap, STDString};

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TextureAtlasMap {
    vptr: *const (),
    pub atlas_map: RefMap<STDString, GamePtr<TextureAtlas>>,
    pub icon_map: Map<FixedString, GamePtr<TextureAtlas>>,
}

/// FIXME: figure out hash for STDString
impl GameHash for STDString {
    fn hash(&self) -> u64 {
        0
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TextureAtlas {
    vptr: *const (),
    pub icons: Map<FixedString, GamePtr<UVValues>>,
    pub path: STDString,
    unk: [u8; 40],
    pub name: FixedString,
    // pub uuid: FixedString,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct UVValues {
    pub u1: f32,
    pub v1: f32,
    pub u2: f32,
    pub v2: f32,
}
