pub mod dice_set;
pub mod die;

use std::{array, collections::HashMap};

use rand::prelude::*;
use raylib::math::Rectangle;

pub use dice_set::DiceSet;
pub use die::{Die, Face};

pub const DICE_COUNT: usize = 5;

pub type BoolSet = [bool; DICE_COUNT];
pub type Indices = Vec<usize>;
pub type RollResult = [Face; DICE_COUNT];

pub fn boolset_to_indices(bool_set: &BoolSet) -> Vec<usize> {
    bool_set
        .iter()
        .enumerate()
        .filter_map(|(i, b)| b.then_some(i))
        .collect()
}

pub fn indices_to_boolset(indices: &Indices) -> BoolSet {
    array::from_fn(|i| indices.contains(&i))
}

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

pub const DEFAULT_SET: DiceSet =
    DiceSet::new([WHITE_DIE, WHITE_DIE, WHITE_DIE, WHITE_DIE, BLACK_DIE]);

pub fn roll_set(dice_set: &DiceSet, rng: &mut impl Rng) -> RollResult {
    array::from_fn(|i| dice_set[i].roll(rng))
}

pub fn result_rectangles(dice_set: &DiceSet, results: &RollResult) -> [Rectangle; DICE_COUNT] {
    array::from_fn(|i| results[i].get_rect(dice_set[i].texture_index()))
}

pub fn replace_rolled_with_blanks(rolled: &BoolSet, results: &RollResult) -> RollResult {
    array::from_fn(|i| if rolled[i] { Face::Blank } else { results[i] })
}

#[derive(Debug)]
pub struct Flash {
    pub face: Face,
    pub matches: [bool; DICE_COUNT],
    pub match_count: usize,
}

fn check_for_freight_train(results: &RollResult) -> Option<Face> {
    for i in 0..(DICE_COUNT - 1) {
        if !results[i].matches(&results[i + 1]) {
            return None;
        }
    }
    Some(*results.iter().find(|f| !f.is_wild()).unwrap_or(&results[4]))
}

fn check_for_flash(results: &RollResult) -> Option<Flash> {
    let mut best_flash = None;
    for i in 0..DICE_COUNT {
        if results[i].is_wild() || results[i] == Face::Blank {
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
            if best_flash
                .as_ref()
                .is_none_or(|f: &Flash| results[i].face_value() > f.face.face_value())
            {
                best_flash = Some(Flash {
                    face: results[i],
                    matches,
                    match_count,
                });
            }
        }
    }
    best_flash
}

pub struct DiceState {
    pub current_roll: RollResult,
    pub dice_last_rolled: BoolSet,
}

#[derive(Debug)]
pub struct MoveOptions {
    // None: do not have to reroll any dice.
    // Some([]): must reroll all dice.
    // Some([...]): must reroll all dice at given indices.
    pub must_reroll: Option<Indices>,

    // For reroll clause, if list is not empty, one of the given dice must be rerolled.
    pub must_reroll_one_of: Indices,

    // Non-scoring dice that can be rerolled. Does not include dice that must be rerolled.
    pub can_reroll: Indices,
}

impl MoveOptions {
    fn reroll_all() -> Self {
        Self {
            must_reroll: Some(Vec::new()),
            must_reroll_one_of: Vec::new(),
            can_reroll: Vec::new(),
        }
    }
    pub fn must_reroll_indices(&self) -> Indices {
        match &self.must_reroll {
            Some(vec) if !vec.is_empty() => vec.clone(),
            Some(_) => (0..DICE_COUNT).collect(),
            None => Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum MoveType {
    FreightTrain(Face),
    Flash(Flash),
}

#[derive(Debug)]
pub struct MoveResult {
    pub current_score: u32,
    // None: wimped out
    pub move_options: Option<MoveOptions>,
    pub move_type: Option<MoveType>,
}

impl MoveResult {
    fn all_scoring(score: u32, move_type: Option<MoveType>) -> Self {
        Self {
            current_score: score,
            move_options: Some(MoveOptions::reroll_all()),
            move_type,
        }
    }
    fn wimped_out() -> Self {
        Self {
            current_score: 0,
            move_options: None,
            move_type: None,
        }
    }
    pub fn rerollable_dice(&self, selected: &Indices) -> Indices {
        if let Some(options) = &self.move_options {
            let mut rerollable = options.can_reroll.clone();
            if let Some(reroll_choice) = options
                .must_reroll_one_of
                .iter()
                .find(|index| selected.contains(*index))
            {
                rerollable.push(*reroll_choice);
            } else {
                rerollable.extend_from_slice(&options.must_reroll_one_of);
            }
            rerollable
        } else {
            Vec::new()
        }
    }
    pub fn valid_reroll(&self, selected: &Indices) -> bool {
        if let Some(options) = &self.move_options {
            // Make sure all dice that must be rerolled are rerolled
            if !options
                .must_reroll_indices()
                .iter()
                .all(|i| selected.contains(i))
            {
                return false;
            }

            // Make sure that exactly ONE die from the set is selected
            if !options.must_reroll_one_of.is_empty()
                && options
                    .must_reroll_one_of
                    .iter()
                    .map(|i| if selected.contains(i) { 1 } else { 0 })
                    .sum::<u32>()
                    != 1
            {
                return false;
            }

            // Make sure that all dice are rerollable
            let rerollable = self.rerollable_dice(selected);
            for die in selected {
                if !options.must_reroll_indices().contains(die) && !rerollable.contains(die) {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    pub fn can_end_turn(&self) -> bool {
        self.move_options
            .as_ref()
            .is_some_and(|o| o.must_reroll.is_none() && o.must_reroll_one_of.is_empty())
    }
}

impl DiceState {
    pub fn new(current_roll: RollResult, dice_last_rolled: BoolSet) -> Self {
        Self {
            current_roll,
            dice_last_rolled,
        }
    }
    pub fn random(dice_set: &DiceSet, rng: &mut impl Rng) -> Self {
        let current_roll = roll_set(dice_set, rng);
        let dice_last_rolled = [true; DICE_COUNT];
        Self::new(current_roll, dice_last_rolled)
    }

    pub fn reroll(&mut self, dice_set: &DiceSet, rng: &mut impl Rng, reroll: Indices) {
        self.dice_last_rolled = indices_to_boolset(&reroll);
        for index in reroll {
            self.current_roll[index] = dice_set[index].roll(rng);
        }
    }

    pub fn rolled_all_dice(&self) -> bool {
        self.dice_last_rolled == [true; DICE_COUNT]
    }

    pub fn is_clearing_flash(&self) -> bool {
        // We are clearing a flash if and only if the non-rolled dice contain a flash.
        // This will be determined by replacing the rolled dice with blank faces, and checking
        // for a flash that way.
        check_for_flash(&replace_rolled_with_blanks(
            &self.dice_last_rolled,
            &self.current_roll,
        ))
        .is_some()
    }

    fn last_rolled_dice_scored(&self) -> bool {
        self.dice_last_rolled
            .into_iter()
            .enumerate()
            .any(|(i, f)| f && self.current_roll[i].is_scoring())
    }

    fn calculate_flash_result(&self, flash: Flash, clearing: bool) -> MoveResult {
        let possible_score = flash.face.face_value() * 10;

        let non_scoring_dice = flash
            .matches
            .iter()
            .enumerate()
            .filter_map(|(i, in_flash)| {
                (!in_flash && !self.current_roll[i].is_scoring()).then_some(i)
            })
            .collect::<Indices>();

        let trace_score: u32 = flash
            .matches
            .iter()
            .enumerate()
            .filter_map(|(i, in_flash)| (!in_flash).then_some(self.current_roll[i].point_value()))
            .sum();

        println!("{flash:?}");
        if flash.match_count == 4 {
            // Reroll clause
            if non_scoring_dice.is_empty() {
                MoveResult {
                    current_score: possible_score + trace_score,
                    move_options: Some(MoveOptions {
                        must_reroll: None,
                        must_reroll_one_of: boolset_to_indices(&flash.matches),
                        can_reroll: Vec::new(),
                    }),
                    move_type: Some(MoveType::Flash(flash)),
                }
            } else {
                assert!(trace_score == 0); // Since there are 4 in the flash, and
                // there are non-scoring dice there must not be any trace dice score.
                MoveResult {
                    current_score: possible_score,
                    move_options: Some(MoveOptions {
                        must_reroll: Some(non_scoring_dice),
                        must_reroll_one_of: boolset_to_indices(&flash.matches),
                        can_reroll: Vec::new(),
                    }),
                    move_type: Some(MoveType::Flash(flash)),
                }
            }
        } else if non_scoring_dice.is_empty() {
            // Lucky! Flash cleared, but must reroll all.
            MoveResult::all_scoring(possible_score + trace_score, Some(MoveType::Flash(flash)))
        } else if clearing {
            if self.last_rolled_dice_scored() {
                // Flash cleared!
                MoveResult {
                    current_score: possible_score + trace_score,
                    move_options: Some(MoveOptions {
                        must_reroll: None,
                        must_reroll_one_of: Vec::new(),
                        can_reroll: non_scoring_dice,
                    }),
                    move_type: Some(MoveType::Flash(flash)),
                }
            } else {
                // Unlucky, wimped out
                MoveResult::wimped_out()
            }
        } else {
            // Must reroll all non-scoring dice.
            // Note: there can be either one or two dice to reroll here.
            MoveResult {
                current_score: possible_score + trace_score,
                move_options: Some(MoveOptions {
                    must_reroll: Some(non_scoring_dice),
                    must_reroll_one_of: Vec::new(),
                    can_reroll: Vec::new(),
                }),
                move_type: Some(MoveType::Flash(flash)),
            }
        }
    }

    fn trace_score(&self) -> u32 {
        self.current_roll.iter().map(|f| f.point_value()).sum()
    }

    fn trace_nonscoring_dice(&self) -> Indices {
        self.current_roll
            .iter()
            .enumerate()
            .filter_map(|(i, f)| (!f.is_scoring()).then_some(i))
            .collect()
    }

    pub fn result(&self) -> MoveResult {
        // Rolled a freight train
        if let Some(face) = check_for_freight_train(&self.current_roll) {
            if face.will_supernova() {
                return MoveResult::wimped_out();
            } else {
                return MoveResult::all_scoring(
                    face.face_value() * 100,
                    Some(MoveType::FreightTrain(face)),
                );
            }
        }

        if self.rolled_all_dice() {
            // Rolled a flash, so must clear it.
            if let Some(flash) = check_for_flash(&self.current_roll) {
                return self.calculate_flash_result(flash, false);
            }

            // Did not flash, so trace score
            let trace_score = self.trace_score();

            if trace_score > 0 {
                return MoveResult {
                    current_score: trace_score,
                    move_options: Some(MoveOptions {
                        must_reroll: None,
                        must_reroll_one_of: Vec::new(),
                        can_reroll: self.trace_nonscoring_dice(),
                    }),
                    move_type: None,
                };
            }
        } else {
            if self.is_clearing_flash() {
                let flash = check_for_flash(&self.current_roll).unwrap();
                return self.calculate_flash_result(flash, true);
            } else if let Some(flash) = check_for_flash(&self.current_roll) {
                // Rolled into a flash that must now be cleared
                return self.calculate_flash_result(flash, false);
            }

            if self.last_rolled_dice_scored() {
                let trace_score = self.trace_score();
                return MoveResult {
                    current_score: trace_score,
                    move_options: Some(MoveOptions {
                        must_reroll: None,
                        must_reroll_one_of: Vec::new(),
                        can_reroll: self.trace_nonscoring_dice(),
                    }),
                    move_type: None,
                };
            }
        }

        MoveResult::wimped_out()
    }
}
