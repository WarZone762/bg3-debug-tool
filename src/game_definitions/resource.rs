use windows::Win32::System::Threading::{CRITICAL_SECTION, SRWLOCK};

use super::{Array, FixedString, GameHash, GamePtr, Map, MultiHashMap, RefMap, STDString};

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ResourceManager {
    field_0: i64,
    pub resources: Map<FixedString, *const ()>,
    pub resource_banks: [*const (); 2],
    pub mesh_proxy_factory: *const (),
    pub visual_factory: *const (),
    pub animation_preload_list: Array<FixedString>,
    pub effect_manager: i64,
    pub effect_factory: i64,
    field_60: i64,
    field_68: i64,
    field_70: i64,
    pub texture_manager: GamePtr<TextureManager>,
    field_80: i64,
    pub sound_manager: *const (),
    pub video_manager: i64,
    pub video_manager2: i64,
    pub game_analytics: i64,
    pub virtual_texture_manager: *const (),
    pub critical_section: CRITICAL_SECTION,
    pub resource_dependencies: RefMap<STDString, *const ()>,
    pub sources: Map<FixedString, *const ()>,
    pub visual_loaders: Array<*const ()>,
    pub genome_animation_managers: Map<FixedString, *const ()>,
    pub blueprint_manager: *const (),
    pub ui_manager: *const (),
    pub ui_manager_swap: *const (),
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TextureManager {
    pub lock: SRWLOCK,
    /// Keys are same as self.textures.values.views
    pub texture_strings: MultiHashMap<GamePtr<TextureDescriptor>, FixedString>,
    pub textures: MultiHashMap<FixedString, GamePtr<GamePtr<TextureDescriptor>>>,
    pad: [u8; 8],
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct TextureDescriptor {
    pub image_data: ImageData,
    pub image_views: Array<GamePtr<ImageViewData>>,
}

impl GameHash for GamePtr<TextureDescriptor> {
    fn hash(&self) -> u64 {
        self.as_usize() as _
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ImageData {
    pub view_descriptors: Array<ViewDescriptor>,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub unk: u16,
    pub mip_count: u16,
    pub array_size: u16,
    pub usage: u16,
    pub dimension: u8,
    pub format: u8,
    pub sample_count: u8,
    pub access: u8,
    pub named_views: u8,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ViewDescriptor {
    pub format: u16,
    pub base_mip: u8,
    pub mip_count: u8,
    pub base_slice: u16,
    pub slice_count: u16,
    pub unk: u16,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ImageViewData {
    unk: u64,
    /// VkImageView in Vulkan
    pub view: u64,
}
