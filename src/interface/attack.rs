use rand::Rng;
use raylib::prelude::*;

use crate::{
    assets::Assets,
    dice::{
        DICE_COUNT, DiceSet, DiceState, Indices, MoveResult, MoveType, boolset_to_indices,
        result_rectangles,
    },
    util::lerp,
};

use super::draw_die;

#[derive(Clone, Copy)]
enum DiceDisplay {
    NonScoring,
    Scoring,
    PartOfFlash,
    PartOfFreightTrain,
    MustReroll,
    MustRerollOneOf,
    Miss,
}

impl DiceDisplay {
    fn get_color(&self, time: f64, index: usize) -> Color {
        match self {
            Self::Scoring => Color::YELLOW,
            Self::PartOfFreightTrain => Color::color_from_hsv(
                (time as f32 * 360.0 + index as f32 * 30.0) % 360.0,
                1.0,
                1.0,
            ),
            Self::PartOfFlash => Color::color_from_normalized(Vector4::new(
                1.0,
                0.65 + f32::sin(time as f32 * 4.0 + index as f32) * 0.25,
                0.0,
                1.0,
            )),
            Self::MustReroll | Self::MustRerollOneOf => Color::color_from_normalized(Vector4::new(
                0.75 + f32::sin(time as f32 * 4.0 + index as f32) * 0.25,
                0.5,
                0.5,
                1.0,
            )),
            Self::Miss => Color::RED,
            _ => Color::WHITE,
        }
    }
}

const INDEX_BUTTON_FIRST: usize = DICE_COUNT;
const INDEX_BUTTON_SECOND: usize = DICE_COUNT + 1;
const MENU_COUNT: usize = DICE_COUNT + 2;

enum MoveButton {
    Reroll(bool),
    Another,
}

struct AttackState {
    dice_set: DiceSet,

    dice_state: DiceState,
    move_result: MoveResult,

    rerollable: Indices,
    menu: Indices,
    selected: Indices,
    pointer: Option<usize>,

    score_banked: u32,
}

impl AttackState {
    pub fn new_round(dice_set: DiceSet, rng: &mut impl Rng) -> Self {
        let dice_state = DiceState::random(&dice_set, rng);
        let move_result = dice_state.result();
        let selected = Vec::new();
        let rerollable = move_result.rerollable_dice(&selected);
        let buttons = if move_result.can_end_turn() {
            vec![INDEX_BUTTON_FIRST, INDEX_BUTTON_SECOND]
        } else {
            vec![INDEX_BUTTON_FIRST]
        };
        let menu = rerollable
            .iter()
            .copied()
            .chain(buttons)
            .collect::<Vec<_>>();
        let pointer = move_result
            .move_options
            .is_some()
            .then_some(INDEX_BUTTON_FIRST);

        Self {
            dice_set,
            dice_state,
            move_result,
            rerollable,
            menu,
            selected,
            pointer,
            score_banked: 0,
        }
    }

    pub fn current_score(&self) -> u32 {
        self.score_banked + self.move_result.current_score
    }

    pub fn can_move(&self) -> bool {
        self.move_result.move_options.is_some()
    }

    pub fn can_end_turn(&self) -> bool {
        self.move_result.can_end_turn()
    }

    pub fn move_buttons(&self) -> Option<(MoveButton, bool)> {
        if let Some(move_options) = self.move_result.move_options.as_ref() {
            if self.can_end_turn() {
                Some((
                    if self.selected.is_empty() {
                        MoveButton::Another
                    } else {
                        MoveButton::Reroll(true)
                    },
                    true,
                ))
            } else if move_options.must_reroll_one_of.is_empty() {
                if matches!(&move_options.must_reroll, Some(a) if a.is_empty()) {
                    Some((MoveButton::Another, false))
                } else {
                    Some((MoveButton::Reroll(true), false))
                }
            } else {
                Some((MoveButton::Reroll(self.currently_valid_reroll()), false))
            }
        } else {
            None
        }
    }
    fn rolling_dice(&self) -> Indices {
        if let Some(move_options) = &self.move_result.move_options {
            let mut reroll = move_options.must_reroll_indices();
            reroll.extend_from_slice(&self.selected);
            reroll.sort_unstable();
            reroll.reverse();

            reroll
        } else {
            Vec::new()
        }
    }

    fn currently_valid_reroll(&self) -> bool {
        self.move_result.valid_reroll(&self.rolling_dice())
    }

    pub fn is_game_over(&self) -> bool {
        self.move_result.move_options.is_none()
    }

    pub fn move_left(&mut self) -> bool {
        if self.is_game_over() {
            return false;
        }
        if let Some(pointer) = &mut self.pointer {
            let menu_index = self.menu.iter().position(|a| *a == *pointer);
            if let Some(menu_index) = menu_index {
                // should never fail
                let prev_pointer = *pointer;
                if menu_index == 0 {
                    *pointer = *self.menu.last().unwrap();
                } else {
                    *pointer = self.menu[menu_index - 1]
                }
                prev_pointer != *pointer
            } else {
                self.pointer = self.menu.first().copied();
                false
            }
        } else {
            false
        }
    }

    pub fn clear_selection(&mut self) -> bool {
        let was_already_empty = self.selected.is_empty();
        self.selected.clear();
        !was_already_empty
    }

    pub fn move_right(&mut self) -> bool {
        if self.is_game_over() {
            return false;
        }
        if let Some(pointer) = &mut self.pointer {
            let menu_index = self.menu.iter().position(|a| *a == *pointer);
            if let Some(menu_index) = menu_index {
                // should never fail
                let prev_pointer = *pointer;
                *pointer = self.menu[(menu_index + 1) % self.menu.len()];
                prev_pointer != *pointer
            } else {
                self.pointer = self.menu.first().copied();
                false
            }
        } else {
            false
        }
    }

    pub fn toggle_die(&mut self) -> Option<bool> {
        if let Some(pointer) = &self.pointer
            && *pointer < DICE_COUNT
        {
            if !self.selected.contains(pointer) {
                self.selected.push(*pointer);
                Some(true)
            } else {
                self.selected.retain(|p| p != pointer);
                Some(false)
            }
        } else {
            None
        }
    }

    pub fn reroll(&mut self, rng: &mut impl Rng) -> Option<bool> {
        if self.can_move() {
            let reroll = self.rolling_dice();

            if !self.move_result.valid_reroll(&reroll) {
                return None;
            }

            self.dice_state.reroll(&self.dice_set, rng, reroll);
            self.move_result = self.dice_state.result();

            self.selected = Vec::new();
            if self.move_result.move_options.is_some() {
                self.rerollable = self.move_result.rerollable_dice(&self.selected);
                let buttons = if self.move_result.can_end_turn() {
                    vec![INDEX_BUTTON_FIRST, INDEX_BUTTON_SECOND]
                } else {
                    vec![INDEX_BUTTON_FIRST]
                };
                self.menu = self.rerollable.iter().copied().chain(buttons).collect();
                self.pointer = Some(INDEX_BUTTON_FIRST);
                Some(true)
            } else {
                self.pointer = None;
                Some(false)
            }
        } else {
            None
        }
    }

    pub fn another(&mut self, rng: &mut impl Rng) {
        let mut new = Self::new_round(self.dice_set, rng);
        new.score_banked = self.current_score();
        *self = new;
    }
}

pub struct AttackInterface {
    dice_set: DiceSet,
    turn: Option<AttackState>,

    round_start: f64,
    beat_start: f64,

    animating: bool,

    anim_rolling: Indices,
    anim_dice_sounds: Vec<String>,
    anim_reveal: [f64; DICE_COUNT],
    anim_color: [DiceDisplay; DICE_COUNT],
    anim_is_clearing_flash: bool,

    anim_score: f64,
    anim_prev_score: u32,
    anim_digits: [Option<f64>; 4],
    anim_scores: Vec<(usize, u32, f64)>,

    anim_suspense: bool,
    anim_bell_pitch: usize,

    anim_shakes: [Option<f64>; DICE_COUNT],
    anim_oscillates: [Option<f64>; DICE_COUNT],

    anim_hover: [Option<f64>; MENU_COUNT],
    anim_select: [Option<f64>; DICE_COUNT],
    anim_deselect: [Option<f64>; DICE_COUNT],

    anim_turn_start: f64,
}

impl AttackInterface {
    pub fn new(dice_set: DiceSet) -> Self {
        Self {
            dice_set,
            turn: None,

            round_start: 0.0,
            beat_start: 0.0,

            animating: false,

            anim_rolling: Vec::new(),
            anim_dice_sounds: Vec::new(),
            anim_reveal: [0.0; DICE_COUNT],
            anim_color: [DiceDisplay::NonScoring; DICE_COUNT],
            anim_is_clearing_flash: false,

            anim_score: 0.0,
            anim_prev_score: 0,
            anim_digits: [None; 4],
            anim_scores: Vec::new(),

            anim_suspense: false,
            anim_bell_pitch: 0,

            anim_shakes: [None; DICE_COUNT],
            anim_oscillates: [None; DICE_COUNT],

            anim_hover: [None; MENU_COUNT],
            anim_select: [None; DICE_COUNT],
            anim_deselect: [None; DICE_COUNT],

            anim_turn_start: 0.0,
        }
    }

    fn set_menu_animation_state(&mut self, turn_end: f64) {
        self.anim_rolling = Vec::new();
        self.anim_is_clearing_flash = false;

        self.anim_hover = [None; MENU_COUNT];
        self.anim_select = [None; DICE_COUNT];
        self.anim_deselect = [None; DICE_COUNT];

        if let Some(pointer) = self.turn.as_ref().and_then(|a| a.pointer) {
            self.anim_hover[pointer] = Some(turn_end);
        }

        self.anim_turn_start = turn_end;
    }

    fn set_rolling_animation_state(&mut self, prev_score: u32, beat_start: f64, rolling: Indices) {
        self.beat_start = beat_start;
        self.animating = true;

        for i in &rolling {
            self.anim_color[*i] = DiceDisplay::NonScoring;
        }
        self.anim_rolling = rolling;
        self.anim_reveal = [0.0; DICE_COUNT];

        self.anim_prev_score = prev_score;
        self.anim_score = prev_score as f64;
        self.anim_digits = [None; 4];
        self.anim_scores.clear();

        self.anim_suspense = false;
        self.anim_bell_pitch = 0;

        self.anim_oscillates = [None; DICE_COUNT];
        self.anim_shakes = [None; DICE_COUNT];

        self.anim_hover = [None; MENU_COUNT];
        self.anim_select = [None; DICE_COUNT];
        self.anim_deselect = [None; DICE_COUNT];
    }

    fn anim_score_target(&self) -> u32 {
        if let Some(turn) = &self.turn {
            if self.animating {
                self.anim_scores.iter().map(|(_, a, _)| *a).sum::<u32>() + self.anim_prev_score
            } else {
                turn.current_score()
            }
        } else {
            0
        }
    }

    fn play_score_sound(assets: &Assets, pitch: usize) {
        let score_sound = assets.get_sound("score");
        score_sound.set_pitch(2.0_f32.powf(1.0 / 12.0).powi(pitch as i32));
        score_sound.play();
    }

    fn tick(&mut self, assets: &Assets, rng: &mut impl Rng, time: f64) {
        let rounded_down_time = (time * 2.0).floor() / 2.0;

        let turn = self.turn.as_mut().unwrap();
        if self.anim_rolling.is_empty() {
            self.animating = false;
            self.anim_score = turn.current_score() as f64;
            if let Some(options) = turn.move_result.move_options.as_ref() {
                if options.must_reroll.as_ref().is_some_and(|v| v.is_empty()) {
                    assets.get_sound("full_clear").play();
                    self.anim_shakes = [Some(rounded_down_time); DICE_COUNT];
                    self.anim_oscillates = [Some(rounded_down_time); DICE_COUNT];
                } else if let Some(MoveType::Flash(flash)) = &turn.move_result.move_type {
                    if turn.dice_state.is_clearing_flash() && flash.match_count == 3 {
                        assets.get_sound("clear").play();
                    } else {
                        assets.get_sound("flash").play();
                    }
                } else {
                    assets.get_sound("safe").play();
                }
                if let Some(must_reroll) = options.must_reroll.as_ref() {
                    for i in must_reroll {
                        self.anim_color[*i] = DiceDisplay::MustReroll;
                        self.anim_shakes[*i] = Some(rounded_down_time);
                    }
                }
            } else {
                self.anim_shakes = [Some(rounded_down_time); DICE_COUNT];
                if turn.dice_state.dice_last_rolled.iter().all(|b| *b) {
                    assets.get_sound("cosmic_wimpout").play();
                } else {
                    assets.get_sound("wimpout").play();
                }
            }

            self.set_canonical_colors();
            self.set_menu_animation_state(rounded_down_time);
            return;
        }

        if self.anim_suspense {
            self.anim_suspense = false;
            assets.get_sound("drumroll_short").play();
            return;
        }

        let die_sound = self.anim_dice_sounds.pop().unwrap_or("die_1".to_string());
        let die_sound = assets.get_sound(&die_sound);
        die_sound.set_pitch(rng.random_range(0.95..=1.05));
        die_sound.play();

        let die_to_reveal = self.anim_rolling.pop().unwrap();
        self.anim_reveal[die_to_reveal] = rounded_down_time;

        let face = turn.dice_state.current_roll[die_to_reveal];
        if face.is_scoring() {
            Self::play_score_sound(assets, self.anim_bell_pitch);
            self.anim_scores
                .push((die_to_reveal, face.point_value(), rounded_down_time));
            self.anim_color[die_to_reveal] = DiceDisplay::Scoring;
            self.anim_bell_pitch += 1;
        }

        match &turn.move_result.move_type {
            Some(MoveType::Flash(flash)) => {
                if self.anim_is_clearing_flash && face.is_scoring() {
                    assets.get_sound("roll_long").stop();
                    self.anim_is_clearing_flash = false;
                    if !flash.matches[die_to_reveal] {
                        assets.get_sound("crash_long").play();
                    }
                }
                if flash.matches[die_to_reveal] {
                    let reveal_count = (0..DICE_COUNT)
                        .filter(|i| !self.anim_rolling.contains(i) && flash.matches[*i])
                        .count();

                    if reveal_count == 3 {
                        assets.get_sound("crash_short").play();
                        for (i, matches) in flash.matches.iter().enumerate() {
                            if *matches && !self.anim_rolling.contains(&i) {
                                self.anim_color[i] = DiceDisplay::PartOfFlash;
                                self.anim_oscillates[i] = Some(rounded_down_time);
                                self.anim_shakes[i] = Some(rounded_down_time);
                                self.anim_scores.retain(|(j, _, _)| *j != i);
                            }
                        }
                        self.anim_scores.push((
                            die_to_reveal,
                            flash.face.face_value() * 10,
                            rounded_down_time,
                        ));
                    } else if reveal_count == 4 {
                        assets.get_sound("reroll_clause").play();
                        for i in 0..DICE_COUNT {
                            if flash.matches[i] {
                                self.anim_color[i] = DiceDisplay::MustRerollOneOf;
                                self.anim_shakes[i] = Some(rounded_down_time);
                            }
                        }
                        if face.is_scoring() {
                            self.anim_scores.pop();
                        }
                    }
                }
            }
            Some(MoveType::FreightTrain(ft_face)) => {
                self.anim_color[die_to_reveal] = DiceDisplay::PartOfFlash;
                self.anim_oscillates[die_to_reveal] = Some(rounded_down_time);
                self.anim_shakes[die_to_reveal] = Some(rounded_down_time);

                if !face.is_scoring() {
                    Self::play_score_sound(assets, self.anim_bell_pitch);
                    self.anim_bell_pitch += 1;
                }
                if self.anim_rolling.is_empty() {
                    self.anim_oscillates = [Some(rounded_down_time); DICE_COUNT];
                    self.anim_shakes = [Some(rounded_down_time); DICE_COUNT];
                    self.anim_scores.push((
                        die_to_reveal,
                        ft_face.face_value() * 100,
                        rounded_down_time,
                    ));
                }
            }
            _ => (),
        }
        if self.anim_rolling.len() == 1
            && (0..DICE_COUNT).all(|i| {
                self.anim_rolling.contains(&i)
                    || matches!(self.anim_color[i], DiceDisplay::NonScoring)
            })
        {
            self.anim_suspense = true;
        } else if self.anim_rolling.is_empty() {
            assets.get_sound("roll_long").stop();
        }
    }

    fn set_canonical_colors(&mut self) {
        self.anim_color = [DiceDisplay::NonScoring; 5];
        if let Some(turn) = &self.turn {
            if let Some(move_options) = &turn.move_result.move_options {
                for i in 0..DICE_COUNT {
                    if turn.dice_state.current_roll[i].is_scoring() {
                        self.anim_color[i] = DiceDisplay::Scoring;
                    }
                }
                for i in move_options.must_reroll.iter().flatten() {
                    self.anim_color[*i] = DiceDisplay::MustReroll;
                }
                for i in &move_options.must_reroll_one_of {
                    self.anim_color[*i] = DiceDisplay::MustRerollOneOf;
                }

                match &turn.move_result.move_type {
                    Some(MoveType::FreightTrain(_)) => {
                        self.anim_color = [DiceDisplay::PartOfFreightTrain; DICE_COUNT]
                    }
                    Some(MoveType::Flash(f)) if f.match_count == 3 => {
                        for i in boolset_to_indices(&f.matches) {
                            self.anim_color[i] = DiceDisplay::PartOfFlash;
                        }
                    }
                    _ => (),
                }
            } else {
                self.anim_color = [DiceDisplay::Miss; DICE_COUNT];
            }
        }
    }

    pub fn update(
        &mut self,
        d: &RaylibDrawHandle,
        assets: &Assets,
        rng: &mut impl Rng,
        time: f64,
    ) -> Option<u32> {
        let prev_anim_score = self.anim_score;
        self.anim_score = lerp(
            self.anim_score,
            self.anim_score_target() as f64,
            0.1 * d.get_frame_time() as f64 * 60.0,
        );
        if self.anim_score.ceil() > prev_anim_score.ceil() {
            let (cas, pas) = (self.anim_score.ceil() as u32, prev_anim_score.ceil() as u32);
            for i in 0..4 {
                if cas / 10_u32.pow(i) % 10 != pas / 10_u32.pow(i) % 10 {
                    self.anim_digits[i as usize] = Some(time);
                }
            }
        }
        if let Some(turn) = &mut self.turn {
            if self.animating {
                if d.is_key_pressed(KeyboardKey::KEY_Z) {
                    self.beat_start = time - 0.5;
                    while !self.anim_rolling.is_empty() {
                        self.tick(assets, rng, time);
                    }
                }
                if time > self.beat_start + 0.5 {
                    self.tick(assets, rng, time);
                    self.beat_start = (time * 2.0).floor() / 2.0;
                }
            } else if let Some(pointer) = turn.pointer {
                if d.is_key_pressed(KeyboardKey::KEY_LEFT) {
                    turn.move_left();
                    if let Some(pointer) = turn.pointer {
                        self.anim_hover[pointer] = Some(time);
                    }
                }
                if d.is_key_pressed(KeyboardKey::KEY_RIGHT) {
                    turn.move_right();
                    if let Some(pointer) = turn.pointer {
                        self.anim_hover[pointer] = Some(time);
                    }
                }
                if d.is_key_pressed(KeyboardKey::KEY_X) {
                    turn.clear_selection();
                }
                if d.is_key_pressed(KeyboardKey::KEY_Z) {
                    let (first_move, can_attack) = turn.move_buttons().unwrap();
                    if pointer == INDEX_BUTTON_SECOND {
                        if can_attack {
                            let total = turn.current_score();
                            self.turn = None;
                            return Some(total);
                        }
                    } else if pointer == INDEX_BUTTON_FIRST {
                        let rolling = turn.rolling_dice();
                        match first_move {
                            MoveButton::Another => {
                                let prev_score = turn.current_score();
                                turn.another(rng);
                                println!("ANOTHER {:#?}", self.turn.as_ref().unwrap().move_result);
                                assets.get_sound("roll_short").play();
                                self.set_rolling_animation_state(
                                    prev_score,
                                    (time * 2.0).ceil() / 2.0,
                                    (0..DICE_COUNT).rev().collect(),
                                );
                            }
                            MoveButton::Reroll(t) if t => {
                                let prev_score = turn.current_score();
                                turn.reroll(rng);
                                self.anim_is_clearing_flash = turn.dice_state.is_clearing_flash();
                                println!(
                                    "REROLLING {:#?}",
                                    self.turn.as_ref().unwrap().move_result
                                );
                                if self.anim_is_clearing_flash {
                                    assets.get_sound("roll_long").play();
                                } else {
                                    assets.get_sound("roll_short").play();
                                };
                                self.set_rolling_animation_state(
                                    prev_score,
                                    (time * 2.0).ceil() / 2.0,
                                    rolling,
                                );
                            }
                            _ => (), // error
                        }
                    } else if let Some(enabled) = turn.toggle_die() {
                        if enabled {
                            self.anim_select[pointer] = Some(time);
                            self.anim_deselect[pointer] = None;
                        } else {
                            self.anim_select[pointer] = None;
                            self.anim_deselect[pointer] = Some(time);
                        }
                    }
                }
            } else if d.is_key_pressed(KeyboardKey::KEY_Z) {
                // missed, so must continue
                self.turn = None;
                return Some(0);
            }
        } else if d.is_key_pressed(KeyboardKey::KEY_Z) {
            self.turn = Some(AttackState::new_round(self.dice_set, rng));
            self.round_start = time;
            self.set_rolling_animation_state(
                0,
                (time * 2.0).ceil() / 2.0,
                (0..DICE_COUNT).rev().collect(),
            );

            println!("{:#?}", self.turn.as_ref().unwrap().move_result);
            assets.get_sound("roll_short").play();
        }

        None
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        assets: &Assets,
        time: f64,
        frame_count: usize,
        font: &Font,
        rng: &mut impl Rng,
    ) {
        if let Some(turn) = &self.turn {
            let turn_elapsed = time - self.round_start;
            let turn_anim = 0.5_f32.powf(turn_elapsed as f32 * 8.0);

            d.draw_texture(
                assets.get_texture("dice_background"),
                0,
                -20 - (128.0 * turn_anim.powi(2)) as i32,
                Color::WHITE,
            );

            d.draw_texture(
                assets.get_texture("cyber1"),
                25,
                25 - (120.0 * turn_anim) as i32,
                Color::WHITE,
            );
            d.draw_texture(
                assets.get_texture("cyber1_flipped"),
                310,
                25 - (120.0 * turn_anim) as i32,
                Color::WHITE,
            );
            d.draw_texture(
                assets.get_texture("cyber2"),
                57,
                36 - (100.0 * turn_anim) as i32,
                Color::new(255, 255, 255, 127),
            );

            for (i, rect) in result_rectangles(&self.dice_set, &turn.dice_state.current_roll)
                .into_iter()
                .enumerate()
            {
                if self.anim_rolling.contains(&i) {
                    draw_die(
                        d,
                        assets,
                        84.0 + i as f32 * 50.0 + rng.random_range(-4.0..=4.0),
                        4.0 + rng.random_range(-4.0..=4.0) - turn_anim * (80.0 + i as f32 * 20.0),
                        &self.dice_set[i].face_rect(i + frame_count),
                        Color::WHITE,
                        Color::WHITE,
                        1.0,
                    );
                } else {
                    let reveal_elapsed = time - self.anim_reveal[i];

                    let hover = self.anim_hover[i]
                        .map_or(0.0, |time_start| 0.5_f64.powf((time - time_start) * 16.0));

                    let (anim_shake_x, anim_shake_y) =
                        self.anim_shakes[i].map_or((0.0, 0.0), |time_start| {
                            let elapsed = time - time_start;

                            (
                                (rng.random_range(-4.0..4.0) * 0.5_f64.powf(elapsed * 6.0)).round(),
                                (rng.random_range(-4.0..4.0) * 0.5_f64.powf(elapsed * 6.0)).round(),
                            )
                        });
                    let anim_oscillate_y = self.anim_oscillates[i].map_or(0.0, |time_start| {
                        let elapsed = time - time_start;
                        (f64::sin(time * 8.0 + i as f64 * 0.5)
                            * (0.8_f64.powf(elapsed * 3.0) * 7.0).floor())
                        .round()
                    });

                    let x_offset =
                        (rng.random_range(-4.0..4.0) * 0.5_f64.powf(reveal_elapsed * 4.0)).round()
                            + self.anim_select[i].map_or(0.0, |_| rng.random_range(-1.0..1.0))
                            + anim_shake_x;

                    let y_offset =
                        (rng.random_range(-4.0..4.0) * 0.5_f64.powf(reveal_elapsed * 4.0)).round()
                            - 0.5_f64.powf(reveal_elapsed * 5.0) * 30.0
                            + anim_shake_y
                            + anim_oscillate_y
                            - self.anim_select[i].map_or(0.0, |time_start| {
                                let anim = 0.5_f64.powf((time - time_start) * 10.0);
                                15.0 - (anim * 15.0)
                                    - f64::cos((time - time_start) * 4.0 + i as f64)
                                        * (1.0 - anim)
                                        * 5.0
                                    + rng.random_range(-1.0..1.0)
                            })
                            - hover * 3.0;

                    let squish_in = (0.5_f64.powf(reveal_elapsed * 5.0) * 0.2)
                        * if !matches!(self.anim_color[i], DiceDisplay::NonScoring) {
                            1.0
                        } else {
                            -0.5
                        };

                    let squish = f64::clamp(
                        1.0 - squish_in
                            + self.anim_shakes[i]
                                .map_or(0.0, |time_start| 0.5_f64.powf((time - time_start) * 5.0))
                                * 0.2
                            - self.anim_oscillates[i]
                                .map_or(0.0, |time_start| 0.5_f64.powf((time - time_start) * 5.0))
                                * 0.4
                            - hover * 0.05
                            - self.anim_select[i].map_or(0.0, |time_start| {
                                0.5_f64.powf((time - time_start) * 20.0) * 0.04
                            })
                            + self.anim_deselect[i].map_or(0.0, |time_start| {
                                0.5_f64.powf((time - time_start) * 20.0) * 0.08
                            }),
                        0.01,
                        10.0,
                    );
                    draw_die(
                        d,
                        assets,
                        84.0 + i as f32 * 50.0 + x_offset as f32,
                        54.0 + y_offset as f32,
                        &rect,
                        self.anim_color[i].get_color(time, i),
                        Color::WHITE,
                        squish as f32,
                    );
                }
            }

            let score = self.anim_score.ceil() as u32;
            if self.animating || turn.move_result.move_options.is_some() {
                for i in 0..4 {
                    let power_of_ten = 10_u32.pow(i);
                    let digit = (score / power_of_ten) % 10;

                    d.draw_texture_rec(
                        assets.get_texture("bignumbers"),
                        Rectangle::new(digit as f32 * 20.0, 0.0, 20.0, 40.0),
                        Vector2::new(
                            590.0 - 24.0 * i as f32,
                            30.0 - turn_anim.powi(2) * (120.0 - 20.0 * i as f32)
                                - self.anim_digits[i as usize]
                                    .map_or(0.0, |end| 0.5_f64.powf((time - end) * 10.0) * 4.0)
                                    as f32,
                        ),
                        if score >= power_of_ten {
                            Color::WHITE
                        } else {
                            Color::GRAY
                        },
                    )
                }
            } else {
                const OFFSETS: [f32; 4] = [12.0, 12.0, 11.0, 10.0];
                for (i, o) in OFFSETS.iter().enumerate() {
                    d.draw_texture_rec(
                        assets.get_texture("bignumbers"),
                        Rectangle::new(*o * 20.0, 0.0, 20.0, 40.0),
                        Vector2::new(
                            590.0 - 24.0 * i as f32 + rng.random_range(-2.0..2.0),
                            30.0 + rng.random_range(-2.0..2.0),
                        ),
                        Color::RED,
                    )
                }
            }

            let turn_end_elapsed = (!self.animating).then_some(time - self.anim_turn_start);

            for (index, plus_score, start_time) in &self.anim_scores {
                let time_elapsed = time - *start_time;
                let x = 70 + *index as i32 * 50;
                let y = 10.0 + 40.0 * 0.5_f64.powf(time_elapsed * 8.0);

                if turn_end_elapsed.is_none_or(|t| t < 2.0) {
                    let y = y + turn_end_elapsed
                        .map_or(0.0, |t| 1.0 - ((t - 1.0).max(0.0).powi(2)) * 40.0);
                    d.draw_text_ex(
                        font,
                        &format!("+{plus_score}"),
                        Vector2::new(x as f32, y.round() as f32),
                        16.0,
                        1.0,
                        Color::WHITE,
                    );
                }
            }

            if let Some(turn_end_elapsed) = turn_end_elapsed {
                if let Some(pointer) = turn.pointer {
                    let anim_in = 0.5_f32.powf(turn_end_elapsed as f32 * 8.0);

                    fn get_menu_x(i: usize) -> f32 {
                        if i == INDEX_BUTTON_FIRST {
                            392.0
                        } else if i == INDEX_BUTTON_SECOND {
                            462.0
                        } else {
                            84.0 + (50 * i) as f32
                        }
                    }

                    let pointer_x = get_menu_x(pointer);

                    for i in &turn.menu {
                        d.draw_texture(
                            assets.get_texture("hand"),
                            get_menu_x(*i) as i32 - 16,
                            82,
                            Color::color_from_hsv(0.0, 0.0, 0.2 - 0.2 * anim_in),
                        );
                    }

                    let (first_button, can_attack) = turn.move_buttons().unwrap();
                    let (first_tex_y, first_enabled) = match first_button {
                        MoveButton::Reroll(e) => (64.0, e),
                        MoveButton::Another => (32.0, true),
                    };

                    d.draw_texture_rec(
                        assets.get_texture("battle_buttons"),
                        Rectangle::new(0.0, first_tex_y, 64.0, 32.0),
                        Vector2::new(
                            360.0,
                            40.0 + f32::sin(time as f32 * 2.0) * 3.0 - anim_in * 100.0,
                        ),
                        if first_enabled {
                            Color::WHITE
                        } else {
                            Color::new(255, 255, 255, 127)
                        },
                    );

                    d.draw_texture_rec(
                        assets.get_texture("battle_buttons"),
                        Rectangle::new(0.0, 0.0, 64.0, 32.0),
                        Vector2::new(
                            430.0,
                            40.0 + f32::cos(time as f32 * 2.0) * 3.0 - anim_in * 200.0,
                        ),
                        if can_attack {
                            if time % 0.25 < 0.125 {
                                Color::YELLOW
                            } else {
                                Color::WHITE
                            }
                        } else {
                            Color::new(255, 255, 255, 127)
                        },
                    );

                    d.draw_texture(
                        assets.get_texture("hand"),
                        pointer_x as i32 - 16,
                        82,
                        Color::color_from_hsv(0.0, 0.0, 1.0 - anim_in),
                    );
                }
            }
        }
    }
}
