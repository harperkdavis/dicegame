pub struct EnemyInfo {
    pub name: &'static str,
    pub sprite: &'static str,

    // Total HP
    pub health: u32,
    // Flat damage reduction
    pub defense: u32,
    // Amount of damage dealt when attacking
    pub attack: u32,
}

pub const FLESHTHING: EnemyInfo = EnemyInfo {
    name: "FleshThing",
    sprite: "enemy",

    health: 1000,
    defense: 0,
    attack: 10,
};
