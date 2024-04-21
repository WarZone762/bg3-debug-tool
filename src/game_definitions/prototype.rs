use bitflags::bitflags;

use super::{
    Array, DamageType, DiceSizeId, FixedString, GamePtr, Guid, MultiHashMap, STDString,
    TranslatedString,
};

#[derive(Debug)]
#[repr(C)]
pub(crate) struct SpellPrototypeManager {
    vptr: *const (),
    pub spells: MultiHashMap<FixedString, GamePtr<SpellPrototype>>,
    pub combat_ai_override_spells: MultiHashMap<FixedString, GamePtr<SpellPrototype>>,
    pub spell_names: Array<FixedString>,
    pub initialized: bool,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct SpellPrototype {
    pub stats_object_index: i32,
    pub spell_type_id: SpellType,
    pub spell_id: FixedString,
    pub spell_school: u8,
    pub spell_flags: SpellFlags,
    pub spell_action_type: u8,
    pub spell_animation_type: u8,
    pub spell_jump_type: u8,
    pub spell_hit_animation_type: u8,
    pub spell_animation_intent_type: u8,
    pub hit_animation_type: u8,
    pub line_of_sight_flags: u32,
    pub cinematic_arena_flags: u32,
    pub cinematic_arena_timeline_override: Guid,
    pub spell_category: u32,
    pub level: i32,
    pub power_level: i32,
    pub has_memory_cost: bool,
    pub spell_container_id: FixedString,
    pub recharge_values_from: i32,
    pub recharge_values_to: i32,
    pub dive_value: DiceSizeId,
    pub cooldown: i8,
    pub weapon_types: u32,
    pub description: DescriptionInfo,
    pub ai_flags: u8,
    field_101: u8,
    pub damage_type: DamageType,
    pub parent_prototype: GamePtr<SpellPrototype>,
    pub child_prototypes: Array<GamePtr<SpellPrototype>>,
    pub use_cost_groups: Array<UseCostGroup>,
    pub ritual_cost_groups: Array<UseCostGroup>,
    pub dual_wielding_use_cost_groups: Array<UseCostGroup>,
    pub hit_cost_groups: Array<UseCost>,
    pub use_costs: Array<UseCost>,
    pub dual_wielding_use_costs: Array<UseCost>,
    pub ritual_costs: Array<UseCost>,
    pub verbal_intent: u32,
    pub spell_animation: Animation,
    pub dual_wielding_animation: Animation,
    pub prepare_effect: FixedString,
    pub prepare_sound: FixedString,
    pub prepare_loop_sound: FixedString,
    pub cast_sound: FixedString,
    pub cast_sound_type: u8,
    field_299: u8,
    pub sheathing: u8,
    pub alternative_cast_text_events: Array<FixedString>,
    pub source_limb_index: i8,
    pub container_spells: Array<FixedString>,
    pub trajectories: Array<Array<FixedString>>,
    pub requirement_events: u32,
    field_2e0: MultiHashMap<u8, *const ()>,
    pub item_wall: FixedString,
    pub interrupt_prototype: FixedString,
    pub combat_ai_override_spell: FixedString,
    pub combat_ai_override_spells: Array<FixedString>,
    pub steer_speed_multiplier: f32,
}

#[derive(Debug)]
#[repr(u8)]
pub(crate) enum SpellType {
    Zone = 0x1,
    MultiStrike = 0x2,
    Projectile = 0x3,
    ProjectileStrike = 0x4,
    Rush = 0x5,
    Shout = 0x6,
    Storm = 0x7,
    Target = 0x8,
    Teleportation = 0x9,
    Wall = 0xA,
    Throw = 0xB,
}

bitflags! {
    #[derive(Debug)]
    pub(crate) struct SpellFlags: u64 {
        const HasVerbalComponent = 0x1;
        const HasSomaticComponent = 0x2;
        const IsJump = 0x4;
        const IsAttack = 0x8;
        const IsMelee = 0x10;
        const HasHighGroundRangeExtension = 0x20;
        const IsConcentration = 0x40;
        const AddFallDamageOnLand = 0x80;
        const ConcentrationIgnoresResting = 0x100;
        const InventorySelection = 0x200;
        const IsSpell = 0x400;
        const CombatLogSetSingleLineRoll = 0x800;
        const IsEnemySpell = 0x1000;
        const CannotTargetCharacter = 0x2000;
        const CannotTargetItems = 0x4000;
        const CannotTargetTerrain = 0x8000;
        const IgnoreVisionBlock = 0x10000;
        const Stealth = 0x20000;
        const AddWeaponRange = 0x40000;
        const IgnoreSilence = 0x80000;
        const ImmediateCast = 0x100000;
        const RangeIgnoreSourceBounds = 0x200000;
        const RangeIgnoreTargetBounds = 0x400000;
        const RangeIgnoreVerticalThreshold = 0x800000;
        const NoSurprise = 0x1000000;
        const IsHarmful = 0x2000000;
        const IsTrap = 0x4000000;
        const IsDefaultWeaponAction = 0x8000000;
        const CallAlliesSpell = 0x10000000;
        const TargetClosestEqualGroundSurface = 0x20000000;
        const CannotRotate = 0x40000000;
        const NoCameraMove = 0x80000000;
        const CanDualWield = 0x100000000;
        const IsLinkedSpellContainer = 0x200000000;
        const Invisible = 0x400000000;
        const AllowMoveAndCast = 0x800000000;
        const UNUSED_D = 0x1000000000;
        const Wildshape = 0x2000000000;
        const UNUSED_E = 0x4000000000;
        const UnavailableInDialogs = 0x8000000000;
        const TrajectoryRules = 0x10000000000;
        const PickupEntityAndMove = 0x20000000000;
        const Temporary = 0x40000000000;
        const RangeIgnoreBlindness = 0x80000000000;
        const AbortOnSpellRollFail = 0x100000000000;
        const AbortOnSecondarySpellRollFail = 0x200000000000;
        const CanAreaDamageEvade = 0x400000000000;
        const DontAbortPerforming = 0x800000000000;
        const NoCooldownOnMiss = 0x1000000000000;
        const NoAOEDamageOnLand = 0x2000000000000;
        const IsSwarmAttack = 0x4000000000000;
        const DisplayInItemTooltip = 0x8000000000000;
        const HideInItemTooltip = 0x10000000000000;
        const DisableBlood = 0x20000000000000;
        const IgnorePreviouslyPickedEntities = 0x40000000000000;
        const IgnoreAoO = 0x80000000000000;
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct DescriptionInfo {
    pub display_name: TranslatedString,
    pub icon: FixedString,
    pub description: TranslatedString,
    pub description_params: STDString,
    pub extra_description: TranslatedString,
    pub extra_description_params: STDString,
    pub short_description: TranslatedString,
    pub short_description_params: STDString,
    pub lore_description: TranslatedString,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct Animation {
    pub part_0: [FixedString; 3],
    pub part_6: [FixedString; 3],
    pub part_4: [FixedString; 3],
    pub part_1: [FixedString; 3],
    pub part_5: [FixedString; 3],
    pub part_2: Array<[FixedString; 3]>,
    pub part_3: [FixedString; 3],
    pub part_7: [FixedString; 3],
    pub part_8: [FixedString; 3],
    pub flags: u8,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct UseCostGroup {
    pub resources: Array<Guid>,
    pub amount: f64,
    pub sub_resource_id: i32,
    pub resource_group: Guid,
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct UseCost {
    pub resource: Guid,
    pub amonut: f32,
}
