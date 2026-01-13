use std::array;

use rand::prelude::*;
use raylib::math::Rectangle;

const DICE_COUNT: usize = 5;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Blank,
    TenPoints,
    Two,
    Three,
    Four,
    FivePoints,
    Six,
    WildSun,
}

impl Face {
    const fn texture_index(&self) -> usize {
        match self {
            Self::Blank | Self::TenPoints => 0,
            Self::Two => 2,
            Self::Three | Self::WildSun => 3,
            Self::Four => 4,
            Self::FivePoints => 1,
            Self::Six => 5,
        }
    }

    const fn point_value(&self) -> u32 {
        match self {
            Self::FivePoints => 5,
            Self::TenPoints | Self::WildSun => 10,
            _ => 0,
        }
    }

    const fn face_value(&self) -> u32 {
        match self {
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::FivePoints => 5,
            Self::Six => 6,
            Self::TenPoints => 10,
            _ => 0,
        }
    }

    const fn will_supernova(&self) -> bool {
        matches!(self, Self::TenPoints)
    }

    pub fn get_rect(&self, texture_index: usize) -> Rectangle {
        Rectangle::new(
            self.texture_index() as f32 * 32.0,
            texture_index as f32 * 32.0,
            32.0,
            32.0,
        )
    }

    pub const fn is_wild(&self) -> bool {
        matches!(self, Self::WildSun)
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.is_wild() || other.is_wild() || self == other
    }
}

#[derive(Clone, Copy)]
pub struct Die {
    faces: [Face; 6],
    texture_index: usize,
}

impl Die {
    pub const fn new(faces: [Face; 6], texture_index: usize) -> Self {
        Self {
            faces,
            texture_index,
        }
    }

    pub fn roll(&self, rng: &mut impl Rng) -> Face {
        self.faces[rng.random_range(0..6)]
    }
}

pub type DiceSet = [Die; DICE_COUNT];
pub type RollResult = [Face; DICE_COUNT];

const WHITE_DIE: Die = Die::new(
    [
        Face::TenPoints,
        Face::Two,
        Face::Three,
        Face::Four,
        Face::FivePoints,
        Face::Six,
    ],
    0,
);

const BLACK_DIE: Die = Die::new(
    [
        Face::TenPoints,
        Face::Two,
        Face::WildSun,
        Face::Four,
        Face::FivePoints,
        Face::Six,
    ],
    1,
);

pub const DEFAULT_SET: DiceSet = [BLACK_DIE, WHITE_DIE, WHITE_DIE, WHITE_DIE, WHITE_DIE];

pub fn roll_set(dice_set: &DiceSet, rng: &mut impl Rng) -> RollResult {
    array::from_fn(|i| dice_set[i].roll(rng))
}

pub fn result_rectangles(dice_set: &DiceSet, results: &RollResult) -> [Rectangle; DICE_COUNT] {
    array::from_fn(|i| results[i].get_rect(dice_set[i].texture_index))
}

#[derive(Debug)]
pub struct Score {
    pub points: u32,
    pub scoring: [bool; DICE_COUNT],
    pub wimped_out: bool,
    pub must_reroll: bool,
}

impl Score {
    pub fn wimped_out() -> Self {
        Self {
            points: 0,
            scoring: [false; DICE_COUNT],
            wimped_out: true,
            must_reroll: false,
        }
    }

    pub fn all_scoring(points: u32) -> Self {
        Self {
            points,
            scoring: [true; DICE_COUNT],
            wimped_out: false,
            must_reroll: true,
        }
    }
}

fn check_for_freight_train(results: &RollResult) -> Option<Face> {
    for i in 0..(DICE_COUNT - 1) {
        if !results[i].matches(&results[i + 1]) {
            return None;
        }
    }
    Some(*results.iter().find(|f| !f.is_wild()).unwrap_or(&results[4]))
}

fn check_for_flash(results: &RollResult) -> Option<(Face, [bool; DICE_COUNT])> {
    for i in 0..DICE_COUNT {
        if results[i].is_wild() {
            continue;
        }

        let mut match_count = 0;
        let mut matches = [false; DICE_COUNT];

        for j in 0..DICE_COUNT {
            if results[i].matches(&results[j]) {
                match_count += 1;
                matches[j] = true;
            }
        }

        if match_count >= 3 {
            matches[i] = true;
            return Some((results[i], matches));
        }
    }
    None
}

pub fn score_faces(results: &RollResult) -> Score {
    if let Some(face) = check_for_freight_train(results) {
        if face.will_supernova() {
            return Score::wimped_out();
        } else {
            return Score::all_scoring(face.face_value() * 100);
        }
    }

    if let Some((face, matches)) = check_for_flash(results) {
        let mut points = face.face_value() * 10;
        let mut any_did_not_score = true;
        let mut scoring = [false; DICE_COUNT];

        for (i, did_match) in matches.into_iter().enumerate() {
            if did_match {
                scoring[i] = true;
                continue;
            }
            if results[i].point_value() > 0 {
                scoring[i] = true;
                points += results[i].point_value();
                any_did_not_score = false;
            }
        }

        return Score {
            points,
            scoring,
            must_reroll: !any_did_not_score,
            wimped_out: false,
        };
    }

    let mut points = 0;
    let mut scoring = [false; DICE_COUNT];

    for (i, face) in results.iter().enumerate() {
        if face.point_value() > 0 {
            points += face.point_value();
            scoring[i] = true;
        }
    }

    if points > 0 {
        return Score {
            points,
            scoring,
            must_reroll: false,
            wimped_out: false,
        };
    }

    Score::wimped_out()
}
