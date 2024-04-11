use x86_64::registers::segmentation::Segment64;

use super::{glm, Array, FixedString, GamePtr, Map, OverrideableProperty, STDString, Transform};
use crate::info;

#[derive(Debug)]
#[repr(C)]
pub(crate) struct GlobalTemplateManager {
    vmt: *const (),
    tepmlates: Map<FixedString, GamePtr<GameObjectTemplate>>,
    banks: [GamePtr<GlobalTemplateBank>; 2],
}
impl GlobalTemplateManager {
    pub fn global_template_bank(&self) -> GamePtr<GlobalTemplateBank> {
        unsafe {
            let tls = **(x86_64::registers::segmentation::GS::read_base().as_ptr::<u8>().add(0x58)
                as *const *const u64);
            let slot = *(tls as *const u8).add(8);
            info!("{slot}");
            self.banks[slot as usize]
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct GlobalTemplateBank {
    vmt: *const (),
    pub templates: Map<FixedString, GamePtr<GameObjectTemplate>>,
    field_20: Array<*const ()>,
    field_30: Array<*const ()>,
    field_40: Array<*const ()>,
    field_50: Array<*const ()>,
    field_60: i32,
    field_64: i32,
    field_68: FixedString,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct GameObjectTemplate {
    pub vmt: GamePtr<GameObjectTemplateVMT>,
    field_8: u64,
    pub id: FixedString,
    pub template_name: FixedString,
    pub parent_template_id: FixedString,
    pub name: STDString,
    pub group_id: OverrideableProperty<u32>,
    pub level_name: FixedString,
    pad: [u8; 4],
    pub camera_offset: OverrideableProperty<glm::Vec3>,
    pub transform: OverrideableProperty<Transform>,
    pub visual_template: OverrideableProperty<FixedString>,
    pub physics_template: OverrideableProperty<FixedString>,
    pub physics_open_template: OverrideableProperty<FixedString>,
    pub cast_shadow: OverrideableProperty<bool>,
    pub receive_decal: OverrideableProperty<bool>,
    pub allow_receive_decal_when_animated: OverrideableProperty<bool>,
    pub is_reflecting: OverrideableProperty<bool>,
    pub is_shadow_proxy: OverrideableProperty<bool>,
    pub global_deleted_flag: u8,
    pub render_channel: OverrideableProperty<u8>,
    pub parent_template_flags: u8,
    pub file_name: STDString,
}

impl GameObjectTemplate {
    pub fn get_type(&self) -> &FixedString {
        (self.vmt.get_type)(self.into()).as_ref()
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct GameObjectTemplateVMT {
    dtor: fn(GamePtr<GameObjectTemplate>),
    get_name: fn(GamePtr<GameObjectTemplate>, *const ()),
    debug_dump: fn(GamePtr<GameObjectTemplate>, *const ()),
    get_type: fn(GamePtr<GameObjectTemplate>) -> GamePtr<FixedString>,
}
