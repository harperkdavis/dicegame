use std::ops::{Index, IndexMut};

pub const LIMBS: [&str; 5] = ["left_leg", "right_leg", "left_arm", "right_arm", "head"];

pub const LEFT_LEG_INDEX: usize = 0;
pub const RIGHT_LEG_INDEX: usize = 1;
pub const LEFT_ARM_INDEX: usize = 2;
pub const RIGHT_ARM_INDEX: usize = 3;
pub const HEAD_INDEX: usize = 4;
pub const BODY_INDEX: usize = 5;

pub const MAX_LEG_HEALTH: u32 = 85;
pub const MAX_ARM_HEALTH: u32 = 65;
pub const MAX_HEAD_HEALTH: u32 = 100;

pub const MAX_HEALTH_VALUES: [u32; 5] = [
    MAX_LEG_HEALTH,
    MAX_LEG_HEALTH,
    MAX_ARM_HEALTH,
    MAX_ARM_HEALTH,
    MAX_HEAD_HEALTH,
];

#[derive(Clone, Copy)]
pub struct Health {
    pub left_leg: u32,
    pub right_leg: u32,
    pub left_arm: u32,
    pub right_arm: u32,
    pub head: u32,
}

impl Index<usize> for Health {
    type Output = u32;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            LEFT_LEG_INDEX => &self.left_leg,
            RIGHT_LEG_INDEX => &self.right_leg,
            LEFT_ARM_INDEX => &self.left_arm,
            RIGHT_ARM_INDEX => &self.right_arm,
            HEAD_INDEX => &self.head,
            _ => panic!("health index out of range"),
        }
    }
}

impl IndexMut<usize> for Health {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            LEFT_LEG_INDEX => &mut self.left_leg,
            RIGHT_LEG_INDEX => &mut self.right_leg,
            LEFT_ARM_INDEX => &mut self.left_arm,
            RIGHT_ARM_INDEX => &mut self.right_arm,
            HEAD_INDEX => &mut self.head,
            _ => panic!("health index out of range"),
        }
    }
}
impl Health {
    pub fn full() -> Self {
        Self {
            left_leg: MAX_LEG_HEALTH,
            right_leg: MAX_LEG_HEALTH,
            left_arm: MAX_ARM_HEALTH,
            right_arm: MAX_ARM_HEALTH,
            head: MAX_HEAD_HEALTH,
        }
    }

    pub fn total(&self) -> u32 {
        self.left_leg + self.right_leg + self.left_arm + self.right_arm + self.head
    }

    pub fn is_dead(&self) -> bool {
        self.head == 0
    }

    pub fn active(&self) -> [bool; 5] {
        if self.is_dead() {
            [false; 5]
        } else {
            [
                self.left_leg > 0,
                self.right_leg > 0,
                self.left_arm > 0,
                self.right_arm > 0,
                true, // Head determines life/death so if alive will always be active
            ]
        }
    }

    pub fn are_arms_dead(&self) -> bool {
        self.left_arm == 0 && self.right_arm == 0
    }

    pub fn are_legs_dead(&self) -> bool {
        self.left_leg == 0 && self.right_leg == 0
    }

    pub fn targetable_limbs(&self) -> Vec<usize> {
        let mut limbs = (0..=3)
            .filter_map(|limb_index| (self[limb_index] > 0).then_some(limb_index))
            .collect::<Vec<_>>();

        if self.are_arms_dead() || self.are_legs_dead() {
            limbs.push(HEAD_INDEX);
        }

        limbs
    }
}
