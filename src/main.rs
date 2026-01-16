mod assets;
mod dice;

use assets::Assets;
use dice::{
    BoolSet, DEFAULT_SET, DICE_COUNT, DiceSet, DiceState, Face, Indices, MoveResult, MoveType,
    indices_to_boolset, result_rectangles,
};
use rand::{Rng, seq::SliceRandom};
use raylib::prelude::*;

const INTERVALS: [i32; 6] = [0, 4, 7, 12, 19, 24];

#[derive(Default)]
struct InterfaceState {
    dice_set: DiceSet,
    result: Option<(DiceState, MoveResult)>,

    dice_to_roll: Indices,
    pointer_index: Option<usize>,
    currently_rolling: BoolSet,

    last_tick: f64,

    queued_dice_sounds: Vec<usize>,
    animating: bool,
    result_time: [f64; DICE_COUNT],
    colors: [Color; DICE_COUNT],
    scoring_count: usize,
    suspense_ticks: u32,
    suspense_is_long: bool,
}

impl InterfaceState {
    pub fn new(dice_set: DiceSet) -> Self {
        Self {
            dice_set,
            result: None,
            dice_to_roll: Vec::new(),
            currently_rolling: [true; DICE_COUNT],

            ..Default::default()
        }
    }

    pub fn play_dice_sound(assets: &Assets, index: usize, rng: &mut impl rand::Rng) {
        let sound = assets.get_sound(&format!("die_{index}"));
        sound.set_pitch(rng.random_range(0.9..1.1));
        sound.play();
    }

    pub fn play_score_sound(assets: &Assets, index: usize, rng: &mut impl rand::Rng) {
        let semitone = 2.0_f32.powf(1.0 / 12.0);
        let sound = assets.get_sound("score");
        sound.set_pitch(semitone.powi(INTERVALS[index]));
        sound.play();
    }

    fn get_selectable_dice(&self) -> Indices {
        match &self.result {
            Some((
                _,
                MoveResult {
                    move_options: Some(move_options),
                    ..
                },
            )) => {
                let mut available = move_options.can_reroll.clone();
                if let Some(i) = self
                    .dice_to_roll
                    .iter()
                    .find(|i| move_options.must_reroll_one_of.contains(i))
                {
                    available.push(*i);
                } else {
                    available.extend_from_slice(&move_options.must_reroll_one_of);
                }
                available.sort();
                available
            }
            _ => Vec::new(),
        }
    }

    pub fn frame(&mut self, d: &RaylibDrawHandle, assets: &Assets, rng: &mut impl rand::Rng) {
        let mut reset = false;

        if !self.animating {
            let selectable = self.get_selectable_dice();
            if !selectable.is_empty() {
                let pointer_index = self.pointer_index.unwrap();
                let curr_index = selectable
                    .iter()
                    .position(|dice_index| *dice_index == pointer_index)
                    .unwrap_or(*selectable.first().unwrap());
                if d.is_key_pressed(KeyboardKey::KEY_LEFT) {
                    if curr_index == 0 {
                        self.pointer_index = Some(*selectable.last().unwrap());
                    } else {
                        self.pointer_index = Some(selectable[curr_index - 1]);
                    }
                } else if d.is_key_pressed(KeyboardKey::KEY_RIGHT) {
                    self.pointer_index = Some(selectable[(curr_index + 1) % selectable.len()]);
                } else if d.is_key_pressed(KeyboardKey::KEY_X) {
                    if self.dice_to_roll.contains(&pointer_index) {
                        assets.get_sound("select").set_pitch(0.5);
                        assets.get_sound("select").set_volume(0.2);
                        assets.get_sound("select").play();
                        self.dice_to_roll.retain(|i| *i != pointer_index);
                    } else {
                        assets.get_sound("select").set_pitch(1.0);
                        assets.get_sound("select").set_volume(0.2);
                        assets.get_sound("select").play();
                        self.dice_to_roll.push(pointer_index);
                    }
                }

                if self.pointer_index != Some(pointer_index) {
                    assets.get_sound("menu").set_volume(0.2);
                    assets.get_sound("menu").play();
                }
            }

            if d.is_key_pressed(KeyboardKey::KEY_Z)
                && let Some((dice_state, move_options)) = self.result.as_mut()
            {
                self.last_tick = d.get_time();
                let mut reroll = move_options
                    .move_options
                    .as_ref()
                    .map(|f| match &f.must_reroll {
                        Some(vec) if !vec.is_empty() => vec.clone(),
                        Some(_) => (0..DICE_COUNT).collect(),
                        None => Vec::new(),
                    })
                    .unwrap_or_default();
                reroll.append(&mut self.dice_to_roll);

                self.queued_dice_sounds = (1..=reroll.len()).collect::<Vec<_>>();
                self.currently_rolling = indices_to_boolset(&reroll);
                dice_state.reroll(&self.dice_set, rng, reroll);
                let move_result = dice_state.result();
                *move_options = move_result;

                self.result_time = [0.0; DICE_COUNT];
                self.last_tick = (d.get_time() * 2.0).ceil() / 2.0 - 0.325;

                self.animating = true;
            }
        }

        if let Some((dice_state, move_result)) = &self.result {
            while d.get_time() > self.last_tick + 0.5 {
                self.last_tick += 0.5;

                if !self.animating {
                    continue;
                }

                if self.suspense_ticks > 0 {
                    self.suspense_ticks -= 1;
                    if self.suspense_ticks == 0 && !self.suspense_is_long {
                        assets.get_sound("drumroll_short").play();
                        continue;
                    }
                }

                // Reveal next die
                if let Some((i, _)) = self
                    .currently_rolling
                    .into_iter()
                    .enumerate()
                    .find(|(_, b)| *b)
                {
                    self.result_time[i] = self.last_tick - 0.5;
                    self.currently_rolling[i] = false;
                    Self::play_dice_sound(
                        assets,
                        self.queued_dice_sounds.pop().unwrap_or_default(),
                        rng,
                    );

                    let mut reroll_clause = false;

                    if let Some(MoveType::Flash(flash)) = &move_result.move_type {
                        if flash.matches[i] {
                            let num_revealed: u32 =
                                (0..i).map(|j| if flash.matches[j] { 1 } else { 0 }).sum();
                            if num_revealed == 2 {
                                Self::play_score_sound(assets, self.scoring_count, rng);
                                self.scoring_count += 1;
                                assets.get_sound("crash_short").play();
                                for j in 0..=i {
                                    if flash.matches[j] {
                                        self.colors[j] = Color::YELLOW;
                                    }
                                }
                            } else if num_revealed == 3 {
                                assets.get_sound("reroll_clause").play();
                                reroll_clause = true;
                                for j in 0..=i {
                                    if flash.matches[j] {
                                        self.colors[j] = Color::PINK;
                                    }
                                }
                            }
                        }
                    }

                    if dice_state.current_roll[i].is_scoring() && !reroll_clause {
                        Self::play_score_sound(assets, self.scoring_count, rng);
                        self.scoring_count += 1;
                        self.colors[i] = Color::YELLOW;
                    }

                    if i == 3 && self.scoring_count == 0 {
                        self.suspense_ticks = 1;
                        self.suspense_is_long = false;
                    }
                } else {
                    // Play results!
                    if let Some(move_options) = &move_result.move_options {
                        match move_result.move_type {
                            Some(MoveType::FreightTrain(_)) => {
                                assets.get_sound("full_clear").play()
                            }
                            Some(MoveType::Flash(_)) => {
                                if move_options
                                    .must_reroll
                                    .as_ref()
                                    .is_some_and(|v| v.is_empty())
                                {
                                    assets.get_sound("full_clear").play();
                                } else {
                                    if let Some(v) = move_options.must_reroll.as_ref() {
                                        for i in v {
                                            self.colors[*i] = Color::RED;
                                        }
                                    }
                                    assets.get_sound("flash").play();
                                }
                            }
                            _ => assets.get_sound("safe").play(),
                        }
                    } else {
                        if move_result.move_type.is_some() {
                            assets.get_sound("wimpout").play();
                        } else {
                            assets.get_sound("cosmic_wimpout").play();
                        }
                        reset = true;
                    }

                    self.pointer_index = self.get_selectable_dice().first().copied();
                    self.animating = false;
                }
            }
        } else if d.is_key_pressed(KeyboardKey::KEY_Z) {
            assets.get_sound("roll_short").play();
            let dice_state = DiceState::random(&self.dice_set, rng);
            let move_result = dice_state.result();

            self.currently_rolling = [true; DICE_COUNT];
            self.result_time = [0.0; DICE_COUNT];

            self.result = Some((dice_state, move_result));
            self.last_tick = (d.get_time() * 2.0).ceil() / 2.0 - 0.325;
            self.scoring_count = 0;

            self.suspense_ticks = 0;
            self.animating = true;
            self.colors = [Color::WHITE; 5];
            self.queued_dice_sounds = (1..=DICE_COUNT).collect::<Vec<_>>();
            self.queued_dice_sounds.shuffle(rng);
        }
        if reset {
            self.result = None;
        }
    }

    pub fn any_is_rolling(&self) -> bool {
        self.currently_rolling != [false; DICE_COUNT]
    }
}

fn draw_die(
    dd: &mut impl RaylibDraw,
    assets: &Assets,
    x: i32,
    y: i32,
    rect: &Rectangle,
    outer_tint: Color,
    inner_tint: Color,
) {
    dd.draw_texture(assets.get_texture("dice_border"), x, y, outer_tint);
    dd.draw_texture_rec(
        assets.get_texture("dice"),
        rect,
        Vector2::new((x + 4) as f32, (y + 4) as f32),
        inner_tint,
    );
}

fn main() -> eyre::Result<()> {
    let (mut rl, rt) = raylib::init().size(1280, 960).title("Hello, World").build();
    let ra = RaylibAudio::init_audio_device()?;

    let assets = Assets::load(&mut rl, &rt, &ra)?;

    let font = rl
        .load_font(&rt, "assets/fonts/karen_bold.fnt")
        .expect("could not load font");

    let camera = Camera2D {
        target: Vector2::new(320.0 / 2.0, 240.0 / 2.0),
        offset: Vector2::new(320.0, 240.0),
        zoom: 2.0,
        rotation: 0.0,
    };

    assets.get_sound("battle").set_volume(0.2);
    assets.get_sound("battle").play();

    let mut rng = rand::rng();

    let mut state = InterfaceState::new(DEFAULT_SET);
    let mut frame_count = 0;

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&rt);

        let time = d.get_time();
        state.frame(&d, &assets, &mut rng);

        let mut dd = d.begin_mode2D(camera);

        dd.clear_background(Color::WHITE);
        let offset = (dd.get_time() * 32.0) as i32 % 32;
        dd.draw_texture(
            assets.get_texture("battle_background"),
            -offset,
            -offset,
            Color::WHITE,
        );
        dd.draw_text_ex(
            &font,
            "Cosmic Wimpout",
            Vector2::new(10.0, 10.0),
            16.0,
            1.0,
            Color::WHITE,
        );

        for (i, rect) in result_rectangles(
            &state.dice_set,
            &state
                .result
                .as_ref()
                .map(|(r, _)| r.current_roll)
                .unwrap_or([Face::Blank; DICE_COUNT]),
        )
        .into_iter()
        .enumerate()
        {
            if state.currently_rolling[i] || state.dice_to_roll.contains(&i) {
                draw_die(
                    &mut dd,
                    &assets,
                    60 + i as i32 * 50 + rng.random_range(-4..=4),
                    70 + rng.random_range(-4..=4),
                    &state.dice_set[i].face_rect(i + frame_count),
                    Color::WHITE,
                    Color::WHITE,
                );
            } else {
                let time_elapsed = time - state.result_time[i];
                let x_offset =
                    (rng.random_range(-4.0..4.0) * 0.5_f64.powf(time_elapsed * 4.0)).round();
                let y_offset = (rng.random_range(-4.0..4.0) * 0.5_f64.powf(time_elapsed * 4.0))
                    .round()
                    + 0.5_f64.powf(time_elapsed * 5.0) * 20.0;
                draw_die(
                    &mut dd,
                    &assets,
                    60 + i as i32 * 50 + x_offset as i32,
                    60 + y_offset as i32,
                    &rect,
                    state.colors[i],
                    Color::WHITE,
                );
            }
        }

        if !state.animating
            && let Some(index) = state.pointer_index
        {
            dd.draw_texture(
                assets.get_texture("hand"),
                70 + index as i32 * 50,
                120 + ((dd.get_time() * 4.0).sin() * 4.0).round() as i32,
                Color::WHITE,
            );
        }

        frame_count += 1;
    }

    Ok(())
}
