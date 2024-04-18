use std::{mem, ops::Deref};

use x86_64::registers::segmentation::Segment64;

use super::{
    glm, Array, FixedString, GamePtr, Guid, Map, MultiHashMap, MultiHashSet, OverrideableProperty,
    STDString, Transform, TranslatedString,
};
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
pub(crate) enum Template<'a> {
    GameObject(&'a GameObjectTemplate),
    EoCGameObject(&'a EoCGameObjectTemplate),
    Scenery(&'a SceneryTemplate),
    Item(&'a ItemTemplate),
}

impl<'a> From<&'a GameObjectTemplate> for Template<'a> {
    fn from(value: &'a GameObjectTemplate) -> Self {
        let r#type = value.get_type().as_str();
        // possible types
        //
        // CombinedLight
        // LevelTemplate
        // Schematic
        // Spline
        // TileConstruction
        // character
        // constellation
        // constellationHelper
        // decal
        // fogVolume
        // item
        // light
        // lightProbe
        // prefab
        // projectile
        // scenery
        // surface
        // terrain
        // trigger
        unsafe {
            match r#type {
                "item" => Self::Item(mem::transmute(value)),
                "scenery" => Self::Scenery(mem::transmute(value)),
                _ => Self::GameObject(mem::transmute(value)),
            }
        }
    }
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

    pub fn get_real_type(&self) -> &FixedString {
        (self.vmt.get_real_type)(self.into()).as_ref()
    }

    pub fn cast<T>(&self) -> &T {
        unsafe { mem::transmute(self) }
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct GameObjectTemplateVMT {
    dtor: fn(GamePtr<GameObjectTemplate>),
    get_name: fn(GamePtr<GameObjectTemplate>, *const ()),
    debug_dump: fn(GamePtr<GameObjectTemplate>, *const ()),
    get_type: fn(GamePtr<GameObjectTemplate>) -> GamePtr<FixedString>,
    get_real_type: fn(GamePtr<GameObjectTemplate>) -> GamePtr<FixedString>,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct EoCGameObjectTemplate {
    pub base: GameObjectTemplate,
    pub ai_bounds: OverrideableProperty<Array<AIBound>>,
    pub display_name: OverrideableProperty<TranslatedString>,
    pub fadeable: OverrideableProperty<bool>,
    pub see_through: OverrideableProperty<bool>,
    pub collide_with_camera: OverrideableProperty<bool>,
    pub hierarchy_only_fade: OverrideableProperty<bool>,
    pub fade_group: OverrideableProperty<FixedString>,
    pub fade_children: Array<FixedString>,
}

impl Deref for EoCGameObjectTemplate {
    type Target = GameObjectTemplate;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct AIBound {
    pub r#type: i32,
    pub height: f32,
    pub radius2: f32,
    pub min: glm::Vec3,
    pub max: glm::Vec3,
    pub radius: f32,
    field_28: u8,
    pub ai_type: u8,
    field_2a: u8,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct SceneryTemplate {
    pub base: EoCGameObjectTemplate,
    pub cover_amount: OverrideableProperty<bool>,
    pub can_climb_on: OverrideableProperty<bool>,
    pub can_shoot_through: OverrideableProperty<bool>,
    pub walk_through: OverrideableProperty<bool>,
    pub walk_on: OverrideableProperty<bool>,
    pub wadable: OverrideableProperty<bool>,
    pub can_click_through: OverrideableProperty<bool>,
    pub is_pointer_blocker: OverrideableProperty<bool>,
    pub is_blocker: OverrideableProperty<bool>,
    pub is_decorative: OverrideableProperty<bool>,
    pub allow_camera_movement: OverrideableProperty<bool>,
    pub can_shine_through: OverrideableProperty<bool>,
    pub block_aoe_damage: OverrideableProperty<bool>,
    pub shoot_through_type: OverrideableProperty<u8>,
    pub wadable_surface_type: OverrideableProperty<u8>,
    pub reference_in_timeline: bool,
    pub loop_sound: OverrideableProperty<FixedString>,
    pub sound_init_event: OverrideableProperty<FixedString>,
    pub hlod: OverrideableProperty<Guid>,
    pub shadow_physics_proxy: OverrideableProperty<FixedString>,
    pub sound_attenuation: OverrideableProperty<i16>,
}

impl Deref for SceneryTemplate {
    type Target = EoCGameObjectTemplate;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ItemTemplate {
    pub base: SceneryTemplate,
    pub combat_component: CombatComponentTemplate,
    pub inventory_list: OverrideableProperty<Array<FixedString>>,
    pub speaker_groups: GamePtr<MultiHashSet<Guid>>,
    pub icon: OverrideableProperty<FixedString>,
    pub can_be_picked_up: OverrideableProperty<bool>,
    pub can_be_pickpocketed: OverrideableProperty<bool>,
    pub is_dropped_on_death: OverrideableProperty<bool>,
    pub can_be_moved: OverrideableProperty<bool>,
    pub destroyed: OverrideableProperty<bool>,
    pub is_interaction_disabled: OverrideableProperty<bool>,
    pub story_item: OverrideableProperty<bool>,
    pub destroy_with_stack: OverrideableProperty<bool>,
    pub is_platform_owner: OverrideableProperty<bool>,
    pub is_key: OverrideableProperty<bool>,
    pub is_tram: OverrideableProperty<bool>,
    pub is_surface_blocker: OverrideableProperty<bool>,
    pub is_surface_cloud_blocker: OverrideableProperty<bool>,
    pub treasure_on_destroy: OverrideableProperty<bool>,
    pub use_party_level_for_treasure_level: OverrideableProperty<bool>,
    pub unimportant: OverrideableProperty<bool>,
    pub hostile: OverrideableProperty<bool>,
    pub use_on_distance: OverrideableProperty<bool>,
    pub use_remotely: OverrideableProperty<bool>,
    pub physics_follow_animation: OverrideableProperty<bool>,
    pub can_be_improvised_weapon: OverrideableProperty<bool>,
    pub force_affected_by_aura: OverrideableProperty<bool>,
    pub is_blueprint_disabled_by_default: OverrideableProperty<bool>,
    pub exclude_in_difficulty: OverrideableProperty<Array<Guid>>,
    pub only_in_difficulty: OverrideableProperty<Array<Guid>>,
    pub unknawn_display_name: OverrideableProperty<TranslatedString>,
    pub show_attached_spell_descriptons: OverrideableProperty<bool>,
    pub gravity_type: u8,
    pub freeze_gravity: u8,
    pub tooltip: OverrideableProperty<u32>,
    pub stats: OverrideableProperty<FixedString>,
    pub on_use_peace_actions: OverrideableProperty<Array<*const ()>>,
    pub on_destry_actions: OverrideableProperty<Array<*const ()>>,
    pub on_use_description: OverrideableProperty<TranslatedString>,
    pub is_portal: OverrideableProperty<bool>,
    pub attachable_with_click_through: OverrideableProperty<bool>,
    pub scripts: OverrideableProperty<Array<*const ()>>,
    pub script_overrides: OverrideableProperty<MultiHashMap<FixedString, *const ()>>,
    pub anubis_config_name: OverrideableProperty<FixedString>,
    pub script_config_global_parameters: OverrideableProperty<Array<*const ()>>,
    pub constellation_config_name: OverrideableProperty<FixedString>,
    pub constellation_config_global_parameters: OverrideableProperty<Array<*const ()>>,
    pub item_list: OverrideableProperty<Array<*const ()>>,
    pub status_list: OverrideableProperty<Array<FixedString>>,
    pub default_state: OverrideableProperty<FixedString>,
    pub owner: OverrideableProperty<FixedString>,
    pub key: OverrideableProperty<FixedString>,
    pub blood_type: OverrideableProperty<FixedString>,
    pub critical_hit_type: OverrideableProperty<FixedString>,
    pub map_marker_style: OverrideableProperty<FixedString>,
    pub lock_difficulty_class_id: OverrideableProperty<Guid>,
    pub disarm_difficulty_class_id: OverrideableProperty<Guid>,
    pub amount: OverrideableProperty<i32>,
    pub max_stack_amount: OverrideableProperty<i32>,
    pub treasure_level: OverrideableProperty<i32>,
    pub equipment: OverrideableProperty<GamePtr<EquipmentData>>,
    pub drop_sound: OverrideableProperty<FixedString>,
    pub pickup_sound: OverrideableProperty<FixedString>,
    pub use_sound: OverrideableProperty<FixedString>,
    pub equip_sound: OverrideableProperty<FixedString>,
    pub unequip_sound: OverrideableProperty<FixedString>,
    pub inventory_move_sound: OverrideableProperty<FixedString>,
    pub impact_sound: OverrideableProperty<FixedString>,
    pub physics_collision_sound: OverrideableProperty<FixedString>,
    pub use_occlusion: OverrideableProperty<bool>,
    pub blood_surface_type: OverrideableProperty<u8>,
    pub book_type: OverrideableProperty<u8>,
    pub inventory_type: OverrideableProperty<u8>,
    pub display_name_alchemy: OverrideableProperty<TranslatedString>,
    pub description: OverrideableProperty<TranslatedString>,
    pub technical_description: OverrideableProperty<TranslatedString>,
    pub short_description: OverrideableProperty<TranslatedString>,
    pub technical_description_params: OverrideableProperty<STDString>,
    pub short_description_params: OverrideableProperty<STDString>,
    pub permanent_warnings: OverrideableProperty<FixedString>,
    pub container_auto_add_on_pickup: OverrideableProperty<bool>,
    pub container_content_filter_condition: OverrideableProperty<STDString>,
    pub interation_filter_list: GamePtr<MultiHashSet<Guid>>,
    pub interaction_filter_type: OverrideableProperty<u8>,
    pub interaction_filter_requirement: OverrideableProperty<u8>,
    pub active_group_id: OverrideableProperty<FixedString>,
    pub level_override: OverrideableProperty<i32>,
    pub is_source_container: OverrideableProperty<bool>,
    pub is_public_domain: OverrideableProperty<bool>,
    pub ignore_generics: OverrideableProperty<bool>,
    pub allow_summon_generic_use: OverrideableProperty<bool>,
    pub is_portal_prohibited_to_players: OverrideableProperty<bool>,
    pub light_channel: OverrideableProperty<u8>,
    pub equipment_type_id: OverrideableProperty<Guid>,
    pub cinematic_arena_flags: OverrideableProperty<u32>,
    pub timeline_camera_rig_override: OverrideableProperty<Guid>,
    pub material_preset: OverrideableProperty<Guid>,
    pub color_preset: OverrideableProperty<Guid>,
    pub examine_rotation: OverrideableProperty<glm::Vec3>,
}

impl Deref for ItemTemplate {
    type Target = SceneryTemplate;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct EquipmentData {
    pub short_hair: MultiHashMap<Guid, FixedString>,
    pub lond_hair: MultiHashMap<Guid, FixedString>,
    pub wavy_short_hair: MultiHashMap<Guid, FixedString>,
    pub wavy_long_hair: MultiHashMap<Guid, FixedString>,
    pub curly_short_hair: MultiHashMap<Guid, FixedString>,
    pub curly_long_hair: MultiHashMap<Guid, FixedString>,
    pub dread_short_hair: MultiHashMap<Guid, FixedString>,
    pub dread_long_hair: MultiHashMap<Guid, FixedString>,
    pub afro_short_hair: MultiHashMap<Guid, FixedString>,
    pub afro_long_hair: MultiHashMap<Guid, FixedString>,
    pub visuals: MultiHashMap<Guid, Array<FixedString>>,
    pub parent_race: MultiHashMap<Guid, Guid>,
    pub sync_with_parent: MultiHashSet<Guid>,
    pub visual_set: *const (),
    slot_vmt: *const (),
    pub slot: Array<FixedString>,
    slot_junk: *const (),
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct CombatComponentTemplate {
    vmt: *const (),
    pub archetype: OverrideableProperty<FixedString>,
    pub swarm_group: OverrideableProperty<FixedString>,
    pub faction: OverrideableProperty<Guid>,
    pub can_fight: OverrideableProperty<bool>,
    pub can_join_combat: OverrideableProperty<bool>,
    pub combat_group_id: OverrideableProperty<FixedString>,
    pub is_boss: OverrideableProperty<bool>,
    pub stay_in_ai_hints: OverrideableProperty<bool>,
    pub ai_hint: OverrideableProperty<Guid>,
    pub is_inspector: OverrideableProperty<bool>,
    unknown: u8,
    unknawn2: u8,
    pub start_combat_range: OverrideableProperty<f32>,
    pub ai_use_combat_helper: OverrideableProperty<FixedString>,
    pub proxy_owner: OverrideableProperty<Guid>,
    pub proxy_attachment: OverrideableProperty<FixedString>,
}
