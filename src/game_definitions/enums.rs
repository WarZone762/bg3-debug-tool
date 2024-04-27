use game_object::TableValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TableValue)]
#[repr(u8)]
pub(crate) enum DiceSizeId {
    D4 = 0,
    D6 = 1,
    D8 = 2,
    D10 = 3,
    D12 = 4,
    D20 = 5,
    D100 = 6,
    Default = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, TableValue)]
#[repr(u8)]
pub(crate) enum DamageType {
    None = 0,
    Slashing = 1,
    Piercing = 2,
    Bludgeoning = 3,
    Acid = 4,
    Thunder = 5,
    Necrotic = 6,
    Fire = 7,
    Lightning = 8,
    Cold = 9,
    Psychic = 10,
    Poison = 11,
    Radiant = 12,
    Force = 13,
    Sentinel = 14,
}
