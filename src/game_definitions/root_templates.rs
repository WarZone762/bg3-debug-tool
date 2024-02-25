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
    vmt: *const (),
    pub flags: OverrideableProperty<u32>,
    pub tags: *const (),
    pub id: FixedString,
    pub name: STDString,
    pub template_name: FixedString,
    pub parent_template_id: FixedString,
    field_50: i32,
    pub is_global: bool,
    pub is_deleted: bool,
    pad: [u8; 4],
    pub group_id: OverrideableProperty<u32>,
    pub transform: OverrideableProperty<Transform>,
    pub visual_template: OverrideableProperty<FixedString>,
    pub physics_template: OverrideableProperty<FixedString>,
    pub physics_open_template: OverrideableProperty<FixedString>,
    pub cast_shadow: OverrideableProperty<bool>,
    pub receive_decal: OverrideableProperty<bool>,
    pub allow_receive_decal_when_animated: OverrideableProperty<bool>,
    pub is_reflecting: OverrideableProperty<bool>,
    pub is_shadow_proxy: OverrideableProperty<bool>,
    pub render_channel: OverrideableProperty<u8>,
    pub camera_offset: OverrideableProperty<glm::Vec3>,
    pub has_parent_mod_relation: OverrideableProperty<bool>,
    pad2: [u8; 6],
    pub has_gameplay_value: OverrideableProperty<bool>,
    pub file_name: STDString,
}
