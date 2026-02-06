use crate::dice::{COMMON_SET, DEFAULT_SET, DiceSet};

pub struct PartyMemberInfo {
    pub name: &'static str,
    pub sprite: &'static str,

    pub health: u32,
    pub default_dice_set: DiceSet,
}

pub const ENN: PartyMemberInfo = PartyMemberInfo {
    name: "Enn",
    sprite: "girl_torso",

    health: 200,
    default_dice_set: DEFAULT_SET,
};

pub const KUE: PartyMemberInfo = PartyMemberInfo {
    name: "Kue",
    sprite: "girl3",
    health: 150,
    default_dice_set: COMMON_SET,
};
