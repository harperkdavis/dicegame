use std::iter;

use rand::Rng;
use raylib::prelude::*;

use crate::{
    dice::DEFAULT_SET,
    game::{
        battle::{
            Action, Battle, DamageEvent,
            health::{self, HEAD_INDEX, LIMBS, MAX_HEAD_HEALTH, MAX_HEALTH_VALUES},
        },
        content::Cnt,
    },
    res::Res,
    util::{TextOutline, lerp, smooth_min},
};

use super::AttackInterface;

struct QueuedDamage {
    from: usize,
    to: usize,
    amount: u32,
}

pub struct BattleInterface {
    battle: Battle,
    attack: Option<AttackInterface>,
    attack_result: Option<u32>,
    battle_start: f64,
    battle_end: Option<f64>,
    in_intro: bool,

    action_select: usize,
    enemy_select: Option<usize>,
    current_action: Option<(Action, usize)>,
    is_enemy_turn: bool,

    anim_actions_menu: Box<[f64]>,
    anim_actions_hover: [f32; 4],
    anim_enemy_damage: Box<[Option<(f64, u32)>]>,
    anim_enemy_death: Box<[Option<f64>]>,
    anim_party_damage: Box<[Option<(f64, u32)>]>,
    anim_party_death: Box<[Option<f64>]>,

    anim_last_damage: f64,
    anim_party_damage_queue: Vec<DamageEvent>,
}

const ENEMY_X: i32 = 450;
const ENEMY_Y: i32 = 200;
const TWO_BAR_DURATION: f64 = 60.0 / 160.0 * 7.0;

impl BattleInterface {
    pub fn new(time: f64, cnt: Cnt) -> Self {
        let battle = Battle::versus(
            &[&cnt.party["enn"], &cnt.party["kue"]],
            &cnt.enemies["fleshthing"],
        );
        let party_count = battle.party().len();
        let enemy_count = battle.enemies().len();

        Self {
            battle,
            attack: None,
            attack_result: None,
            battle_start: time,
            battle_end: None,
            in_intro: true,

            action_select: 0,
            enemy_select: None,
            current_action: None,
            is_enemy_turn: false,

            anim_actions_menu: iter::repeat_n(0.0, party_count).collect(),
            anim_actions_hover: [0.0; 4],
            anim_enemy_damage: iter::repeat_n(None, enemy_count).collect(),
            anim_enemy_death: iter::repeat_n(None, enemy_count).collect(),
            anim_party_damage: iter::repeat_n(None, party_count).collect(),
            anim_party_death: iter::repeat_n(None, party_count).collect(),

            anim_last_damage: 0.0,
            anim_party_damage_queue: Vec::new(),
        }
    }

    pub fn set_menu_animation_state(&mut self) {
        let party_count = self.battle.party().len();
        let enemy_count = self.battle.party().len();
        self.anim_actions_menu = iter::repeat_n(0.0, party_count).collect();
        self.anim_actions_hover = [0.0; 4];
        self.anim_enemy_damage = iter::repeat_n(None, enemy_count).collect();
        self.anim_party_damage = iter::repeat_n(None, party_count).collect();

        self.attack = None;
        self.attack_result = None;

        self.action_select = 0;
        self.enemy_select = None;

        self.anim_last_damage = 0.0;
        self.anim_party_damage_queue = Vec::new();
    }

    pub fn reset_enemy_animation(&mut self) {
        let enemy_count = self.battle.party().len();
        self.anim_enemy_damage = iter::repeat_n(None, enemy_count).collect();
    }

    pub fn update(&mut self, d: &RaylibDrawHandle, res: &Res, rng: &mut impl Rng, time: f64) {
        if self.battle_end.is_some() {
            return;
        }

        if self.in_intro {
            if time - self.battle_start >= TWO_BAR_DURATION || d.is_key_down(KeyboardKey::KEY_Z) {
                self.in_intro = false;
                self.set_menu_animation_state();
                // battle start needs to be when the music starts or offset by the two bar duration
                if !d.is_key_down(KeyboardKey::KEY_Z) {
                    self.battle_start += TWO_BAR_DURATION;
                }
            }
            return;
        }
        for i in 0..self.battle.party().len() {
            self.anim_actions_menu[i] = lerp(
                self.anim_actions_menu[i],
                if self.battle.current_party_member() == Some(i) {
                    1.0
                } else {
                    0.0
                },
                (0.2 * d.get_frame_time() * 60.0).clamp(0.0, 1.0) as f64,
            );
        }
        for i in 0..4 {
            self.anim_actions_hover[i] = lerp(
                self.anim_actions_hover[i],
                if self.action_select == i { 1.0 } else { 0.0 },
                (0.2 * d.get_frame_time() * 60.0).clamp(0.0, 1.0),
            )
        }

        if self.battle.is_player_turn() {
            if let Some(enemy) = self.enemy_select.as_mut() {
                let max = self.battle.enemies().len() - 1;
                if d.is_key_pressed(KeyboardKey::KEY_UP) || d.is_key_pressed(KeyboardKey::KEY_LEFT)
                {
                    if *enemy == 0 {
                        *enemy = max;
                    } else {
                        *enemy -= 1;
                    }
                }
                if d.is_key_pressed(KeyboardKey::KEY_DOWN)
                    || d.is_key_pressed(KeyboardKey::KEY_RIGHT)
                {
                    if *enemy == max {
                        *enemy = 0;
                    } else {
                        *enemy += 1;
                    }
                }
                if d.is_key_pressed(KeyboardKey::KEY_X) {
                    self.enemy_select = None;
                    res.snd("menu").play();
                } else if d.is_key_pressed(KeyboardKey::KEY_Z) {
                    self.battle.push_action(Action::Attack(*enemy));
                    self.anim_actions_hover = [0.0; 4];
                    res.snd("select").play();
                    self.enemy_select = None;
                }
            } else {
                if d.is_key_pressed(KeyboardKey::KEY_UP) {
                    res.snd("menu").play();
                    if self.action_select == 0 {
                        self.action_select = 2;
                    } else {
                        self.action_select -= 1;
                    }
                }
                if d.is_key_pressed(KeyboardKey::KEY_DOWN) {
                    res.snd("menu").play();
                    if self.action_select == 2 {
                        self.action_select = 0;
                    } else {
                        self.action_select += 1;
                    }
                }
                if d.is_key_pressed(KeyboardKey::KEY_Z) {
                    if self.action_select == 0 {
                        res.snd("select").play();
                        self.enemy_select = Some(0);
                    } else if self.action_select == 1 {
                        res.snd("select").play();
                        self.battle.push_action(Action::Defend);
                    }
                }
            }
        } else if self.is_enemy_turn {
            // enemy turn
            if time > self.anim_last_damage + 1.0 || d.is_key_pressed(KeyboardKey::KEY_Z) {
                self.anim_last_damage = time;
                if let Some(DamageEvent { to, amount, .. }) = self.anim_party_damage_queue.pop() {
                    res.snd("party_hurt").set_pitch(rng.random_range(0.9..1.1));
                    res.snd("party_hurt").play();
                    self.anim_party_damage[to] = Some((time, amount));
                } else {
                    self.is_enemy_turn = false;
                    self.battle.finish_enemy_turn();
                    if self.battle.battle_result().is_some() {
                        let music_progress = time - self.battle_start;
                        let next_end_time = (music_progress / TWO_BAR_DURATION).ceil()
                            * TWO_BAR_DURATION
                            + self.battle_start;
                        self.battle_end = Some(next_end_time);
                    } else {
                        self.set_menu_animation_state();
                    }
                }
            }
        } else if let Some((action, party_member)) = &self.current_action {
            let member = &self.battle.party()[*party_member];
            match action {
                Action::Attack(target) => {
                    if let Some(damage) = &self.attack_result {
                        let attack = self.attack.as_ref().unwrap();
                        if let Some(dat) = attack.damage_apply_time()
                            && time > dat
                            && self.anim_enemy_damage[*target].is_none()
                        {
                            if *damage == 0 {
                                res.snd("miss").set_pitch(rng.random_range(0.9..1.1));
                                res.snd("miss").play();
                            } else {
                                res.snd("enemy_hurt").set_pitch(rng.random_range(0.9..1.1));
                                res.snd("enemy_hurt").play();
                            }

                            self.anim_enemy_damage[*target] = Some((dat, *damage));
                            if self.battle.enemies()[*target].health() == 0 {
                                // TODO: kill sound
                                self.anim_enemy_death[*target] = Some(dat);
                            }
                        }
                        // after
                        if attack.can_advance(time, d.is_key_pressed(KeyboardKey::KEY_Z)) {
                            self.attack = None;
                            self.attack_result = None;
                            self.current_action = None;
                        }
                    } else if let Some(attack) = &mut self.attack {
                        if let Some(damage) = attack.update(d, res, rng, time) {
                            self.attack_result = Some(damage);
                            self.battle.apply_damage(*target, damage);
                        }
                    } else {
                        self.attack = Some(AttackInterface::new_round(
                            res,
                            DEFAULT_SET,
                            rng,
                            *member.health(),
                            time,
                            Vector2::new(ENEMY_X as f32 - 10.0, ENEMY_Y as f32 - 30.0),
                        ));
                    }
                }
                // TODO: add animations and sounds for others
                Action::Defend => self.current_action = None,
                Action::Flee => (),
            }
        } else if let Some(next_action) = self.battle.process_next_action() {
            self.current_action = Some(next_action);
            self.reset_enemy_animation();
        } else {
            self.current_action = None;
            self.is_enemy_turn = true;
            self.anim_party_damage_queue = self.battle.run_enemy_turn(rng);
        }
    }

    pub fn battle_result(&self) -> Option<(bool, f64)> {
        self.battle_end
            .and_then(|a| self.battle.battle_result().map(|b| (b, a)))
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        res: &Res,
        time: f64,
        frame_count: usize,
        rng: &mut impl Rng,
    ) {
        let mut shader = res.sha("battle_background").borrow_mut();
        let battle_background_ocean = res.tex("battle_background_ocean");

        let main_time_loc = shader.get_shader_location("main_time");
        let end_time_loc = shader.get_shader_location("end_time");

        let time_f32 = time as f32 - self.battle_start as f32;
        // smoothly slow down the scrolling
        if let Some(end_time) = self.battle_end {
            shader.set_shader_value(
                main_time_loc,
                smooth_min(
                    time_f32,
                    end_time as f32 + 1.0 - self.battle_start as f32,
                    2.0,
                ),
            );
            let diff = time - (end_time + 2.0);
            shader.set_shader_value(
                end_time_loc,
                if diff < 0.0 {
                    -(1.0 + diff / 2.0).clamp(0.0, 1.0).powi(3) as f32
                } else {
                    diff as f32
                },
            );
        } else {
            shader.set_shader_value(main_time_loc, time_f32);
            shader.set_shader_value(end_time_loc, 0.0_f32);
        }

        let mut sm = d.begin_shader_mode(&mut shader);

        sm.draw_texture_pro(
            battle_background_ocean,
            Rectangle::new(
                0.0,
                0.0,
                battle_background_ocean.width as f32,
                battle_background_ocean.height as f32,
            ),
            Rectangle::new(0.0, 0.0, 1280.0, 960.0),
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );
        drop(sm);

        if self.in_intro {
            let anim_in = f32::exp(((time - self.battle_start) * -2.0) as f32);

            d.draw_line_ex(
                Vector2::new(320.0 + 60.0 + anim_in.sqrt() * 50.0, 0.0),
                Vector2::new(320.0 - 60.0 - anim_in.sqrt() * 50.0, 480.0),
                (1.0 - anim_in) * 60.0
                    + (((time - self.battle_start) / TWO_BAR_DURATION) * 60.0) as f32,
                Color::BLACK,
            );
            d.draw_texture(
                res.tex("enn_battleintro"),
                -70 - (anim_in * 20.0) as i32,
                150 + (anim_in.powi(2) * 400.0) as i32,
                Color::WHITE,
            );
            d.draw_texture(
                res.tex("fleshthing_concept"),
                350 + (anim_in * 50.0) as i32,
                150 + (anim_in.powi(2) * 400.0) as i32,
                Color::WHITE,
            );
            d.draw_rectangle(0, (-80.0 * anim_in) as i32, 640, 80, Color::BLACK);
            d.draw_rectangle(0, 400 + (80.0 * anim_in) as i32, 640, 80, Color::BLACK);

            let anim_out = ((time - self.battle_start) / TWO_BAR_DURATION).powi(10) as f32;
            d.draw_rectangle(
                0,
                0,
                640,
                480,
                Color::new(255, 255, 255, (anim_out * 255.0).min(255.0) as u8),
            );

            return;
        }

        let anim_in_battle = f32::exp(((time - self.battle_start) * -4.0) as f32);

        let text_font = res.fnt("unnamedfont");
        let numbers_font = res.fnt("execution");
        // draw actual battle
        for (i, enemy) in self.battle.enemies().iter().enumerate() {
            let sprite = res.tex(enemy.info().sprite.as_str());
            let enemy_x = ENEMY_X + (anim_in_battle * 200.0) as i32;
            let enemy_y = ENEMY_Y;
            if let Some(start) = self.anim_enemy_death[i] {
                let anim = 0.5_f32.powf((time - start) as f32 * 3.0);
                d.draw_texture(
                    sprite,
                    enemy_x - sprite.width / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                    enemy_y - sprite.height / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                    Color::color_from_hsv(0.0, 1.0, 1.0).alpha(anim),
                );
            } else if let Some((start, damage)) = self.anim_enemy_damage[i] {
                let anim = 0.5_f32.powf((time - start) as f32 * 4.0);
                if damage == 0 {
                    d.draw_texture(
                        sprite,
                        enemy_x - sprite.width / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                        enemy_y - sprite.height / 2,
                        Color::new(255, 255, 255, 127),
                    );
                } else {
                    d.draw_texture(
                        sprite,
                        enemy_x - sprite.width / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                        enemy_y - sprite.height / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                        Color::color_from_hsv(0.0, anim * 0.5, 1.0),
                    );
                }
            } else {
                d.draw_texture(
                    sprite,
                    enemy_x - sprite.width / 2,
                    enemy_y - sprite.height / 2,
                    Color::WHITE,
                );
            }

            if let Some(index) = self.enemy_select
                && i == index
            {
                d.draw_text_ex(
                    text_font,
                    &enemy.info().name.to_uppercase(),
                    Vector2::new(ENEMY_X as f32, ENEMY_Y as f32),
                    16.0,
                    0.0,
                    Color::WHITE,
                );
                d.draw_text_outline(
                    numbers_font,
                    &format!("{}/{}", enemy.health(), enemy.info().health),
                    ENEMY_X as f32,
                    ENEMY_Y as f32 + 20.0,
                    Color::WHITE,
                    Color::BLACK,
                );
            }
        }
        d.draw_texture(
            res.tex("girl2"),
            200 - (anim_in_battle * 200.0) as i32,
            300,
            Color::WHITE,
        );

        for (i, member) in self.battle.party().iter().enumerate() {
            let sprite = res.tex(member.info().sprite_battle.as_str());
            let anim = self.anim_actions_menu[i];
            let anim_y = (anim * 80.0).round() as i32 - (anim_in_battle * 200.0) as i32;
            let x_offset = (i * 160) as i32;
            let char_y = 470 - 128 - anim_y + (f64::sin(time * 2.0) * 4.0).round() as i32
                - (anim * 10.0) as i32;

            if let Some((start, damage)) = self.anim_party_damage[i] {
                let anim = 0.5_f32.powf((time - start) as f32 * 4.0);
                if damage == 0 {
                    d.draw_texture(
                        sprite,
                        20 + (rng.random_range(-4.0..4.0) * anim).round() as i32 + x_offset,
                        char_y,
                        Color::new(255, 255, 255, 127),
                    );

                    d.draw_text_outline(
                        numbers_font,
                        "MISS",
                        50.0 + x_offset as f32,
                        480.0 - 128.0 + anim * 40.0,
                        Color::new(255, 255, 255, 127),
                        Color::new(0, 0, 0, 31),
                    );
                } else {
                    d.draw_texture(
                        sprite,
                        20 + (rng.random_range(-4.0..4.0) * anim).round() as i32 + x_offset,
                        char_y + (rng.random_range(-4.0..4.0) * anim).round() as i32,
                        Color::color_from_hsv(0.0, anim * 0.5, 1.0),
                    );

                    d.draw_text_outline(
                        numbers_font,
                        &format!("-{damage}"),
                        50.0 + x_offset as f32,
                        480.0 - 128.0 + anim * 40.0,
                        Color::RED,
                        Color::BLACK,
                    );
                }
            } else {
                d.draw_texture(sprite, 20 + x_offset, char_y, Color::WHITE);
            }

            d.draw_texture(
                res.tex("ui_charactername"),
                x_offset,
                460 - anim_y,
                Color::new(51, 51, 51, 255),
            );

            d.draw_text_outline(
                text_font,
                &member.info().name.to_uppercase(),
                5.0 + x_offset as f32,
                443.0 - anim_y as f32,
                Color::WHITE,
                Color::BLACK,
            );

            d.draw_texture(
                res.tex("ui/body/default/base"),
                x_offset + 115,
                460 - anim_y,
                Color::WHITE,
            );

            for (i, seg) in LIMBS.iter().enumerate() {
                let health = member.health()[i];
                let max_health = MAX_HEALTH_VALUES[i];
                let limb_color = super::health_color(health, max_health, time);
                d.draw_texture(
                    res.tex(format!("ui/body/default/{seg}").as_str()),
                    x_offset + 115,
                    460 - anim_y,
                    limb_color,
                );
            }

            let text_health_color = Color::lerp(
                &super::health_color(member.health()[HEAD_INDEX], MAX_HEAD_HEALTH, time),
                Color::RED,
                self.anim_party_damage[i]
                    .map(|(t, d)| {
                        if d > 0 {
                            0.5_f32.powf((time - t) as f32 * 4.0)
                        } else {
                            0.0
                        }
                    })
                    .unwrap_or_default(),
            );

            d.draw_text_outline(
                numbers_font,
                &member.total_health().to_string(),
                x_offset as f32 + 4.0,
                463.0 - anim_y as f32,
                text_health_color,
                Color::BLACK,
            );

            d.draw_text_outline(
                res.fnt("execution_small"),
                "/400",
                x_offset as f32 + 35.0,
                463.0 - anim_y as f32,
                Color::WHITE,
                Color::BLACK,
            );

            d.draw_rectangle(x_offset, 480 - anim_y, 160, 80, Color::BLACK);

            let action_buttons = res.tex("battle_actions");

            for j in 0..4 {
                d.draw_texture_rec(
                    action_buttons,
                    Rectangle::new(0.0, 15.0 * j as f32, 15.0, 15.0),
                    Vector2::new(
                        20.0 + x_offset as f32
                            + f32::sin(time as f32 * 4.0) * self.anim_actions_hover[j] * 3.0,
                        482.0 - anim_y as f32 + 20.0 * j as f32,
                    ),
                    if self.action_select == j {
                        Color::WHITE
                    } else {
                        Color::new(63, 63, 63, 255)
                    },
                );
                d.draw_texture_rec(
                    action_buttons,
                    Rectangle::new(15.0, 15.0 * j as f32, 85.0, 15.0),
                    Vector2::new(
                        50.0 + x_offset as f32 + self.anim_actions_hover[j] * 5.0,
                        482.0 - anim_y as f32 + 20.0 * j as f32,
                    ),
                    if self.action_select == j {
                        Color::WHITE
                    } else {
                        Color::new(63, 63, 63, 255)
                    },
                );
            }
        }

        if let Some(attack) = &self.attack {
            attack.draw(d, res, time, frame_count, numbers_font, rng);
        }

        if let Some(end_time) = self.battle_end {
            let reveal_start = end_time + (7.0 / 8.0);
            let reveal_count = ((time - reveal_start).max(0.0) * 8.0).ceil() as usize;

            let elapsed = time - end_time;
            let full_anim = if elapsed > 2.0 {
                0.5_f64.powf((elapsed - 2.0) * 2.0)
            } else {
                0.0
            };

            for i in 0..reveal_count.min(9) {
                let revealed_at = reveal_start + i as f64 / 8.0;
                let letter_elapsed = time - revealed_at;
                let letter_anim = 0.5_f64.powf(letter_elapsed * 8.0) as f32;
                d.draw_texture_rec(
                    res.tex("execution"),
                    Rectangle::new(i as f32 * 22.0, 0.0, 22.0, 42.0),
                    Vector2::new(
                        110.0 + 50.0 * i as f32 + rng.random_range(-4.0..=4.0) * full_anim as f32,
                        60.0 + letter_anim * 10.0
                            + full_anim as f32 * 20.0
                            + rng.random_range(-4.0..=4.0) * full_anim as f32
                            + if elapsed > 2.0 {
                                f32::sin(time as f32 * 4.0 + i as f32) * 3.0
                            } else {
                                0.0
                            },
                    ),
                    Color::WHITE,
                )
            }
        }
    }
}
