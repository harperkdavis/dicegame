use std::iter;

use rand::Rng;
use raylib::prelude::*;

use crate::{
    dice::DEFAULT_SET,
    game::{
        battle::{Action, Battle},
        content::Cnt,
    },
    res::Res,
    util::lerp,
};

use super::AttackInterface;

pub struct BattleInterface {
    battle: Battle,
    attack: Option<AttackInterface>,
    attack_result: Option<u32>,
    battle_start: f64,

    action_select: usize,
    enemy_select: Option<usize>,
    current_action: Option<(Action, usize)>,
    is_enemy_turn: bool,

    anim_actions_menu: Box<[f64]>,
    anim_enemy_damage: Box<[Option<(f64, u32)>]>,
    anim_party_damage: Box<[Option<(f64, u32)>]>,

    anim_last_damage: f64,
    anim_party_damage_queue: Vec<(usize, u32)>,
}

const ENEMY_X: i32 = 450;
const ENEMY_Y: i32 = 200;

impl BattleInterface {
    pub fn new(time: f64, cnt: Cnt) -> Self {
        let battle = Battle::versus(&cnt.party["enn"], &cnt.enemies["fleshthing"]);
        let party_count = battle.party().len();
        let enemy_count = battle.enemies().len();

        Self {
            battle,
            attack: None,
            attack_result: None,
            battle_start: time,

            action_select: 0,
            enemy_select: None,
            current_action: None,
            is_enemy_turn: false,

            anim_actions_menu: iter::repeat_n(0.0, party_count).collect(),
            anim_enemy_damage: iter::repeat_n(None, enemy_count).collect(),
            anim_party_damage: iter::repeat_n(None, party_count).collect(),

            anim_last_damage: 0.0,
            anim_party_damage_queue: Vec::new(),
        }
    }

    pub fn set_menu_animation_state(&mut self) {
        let party_count = self.battle.party().len();
        let enemy_count = self.battle.party().len();
        self.anim_actions_menu = iter::repeat_n(0.0, party_count).collect();
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
                if d.is_key_pressed(KeyboardKey::KEY_Z) && self.action_select == 0 {
                    res.snd("select").play();
                    self.enemy_select = Some(0);
                }
            }
        } else if self.is_enemy_turn {
            // enemy turn
            if time > self.anim_last_damage + 1.0 {
                self.anim_last_damage = time;
                if let Some((target, damage)) = self.anim_party_damage_queue.pop() {
                    res.snd("party_hurt").set_pitch(rng.random_range(0.9..1.1));
                    res.snd("party_hurt").play();
                    self.anim_party_damage[target] = Some((time, damage));
                } else {
                    self.is_enemy_turn = false;
                    self.battle.finish_enemy_turn();
                    self.set_menu_animation_state();
                }
            }
        } else if let Some((action, party_member)) = &self.current_action {
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
                            self.anim_enemy_damage[*target] = Some((dat, *damage))
                        }
                        // after
                        if attack.can_advance(time) {
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
                            time,
                            Vector2::new(ENEMY_X as f32 - 10.0, ENEMY_Y as f32 - 30.0),
                        ));
                    }
                }
                _ => todo!(),
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

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        res: &Res,
        time: f64,
        frame_count: usize,
        rng: &mut impl Rng,
    ) {
        let font = res.fnt("default");
        // draw actual battle
        for (i, enemy) in self.battle.enemies().iter().enumerate() {
            let sprite = res.tex(enemy.info().sprite.as_str());
            if let Some((start, damage)) = self.anim_enemy_damage[i] {
                let anim = 0.5_f32.powf((time - start) as f32 * 4.0);
                if damage == 0 {
                    d.draw_texture(
                        sprite,
                        ENEMY_X - sprite.width / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                        ENEMY_Y - sprite.height / 2,
                        Color::new(255, 255, 255, 127),
                    );
                } else {
                    d.draw_texture(
                        sprite,
                        ENEMY_X - sprite.width / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                        ENEMY_Y - sprite.height / 2 + (rng.random_range(-4.0..4.0) * anim) as i32,
                        Color::color_from_hsv(0.0, anim * 0.5, 1.0),
                    );
                }
            } else {
                d.draw_texture(
                    sprite,
                    ENEMY_X - sprite.width / 2,
                    ENEMY_Y - sprite.height / 2,
                    Color::WHITE,
                );
            }

            if let Some(index) = self.enemy_select {
                if i == index {
                    d.draw_text_ex(
                        font,
                        &enemy.info().name.to_uppercase(),
                        Vector2::new(ENEMY_X as f32, ENEMY_Y as f32),
                        16.0,
                        0.0,
                        Color::WHITE,
                    );
                    d.draw_text_ex(
                        font,
                        &format!("{} / {}", enemy.health(), enemy.info().health),
                        Vector2::new(ENEMY_X as f32, ENEMY_Y as f32 + 20.0),
                        16.0,
                        0.0,
                        Color::WHITE,
                    );
                }
            }
        }
        d.draw_texture(res.tex("girl2"), 100, 300, Color::WHITE);

        for (i, member) in self.battle.party().iter().enumerate() {
            let sprite = res.tex(member.info().sprite.as_str());
            let anim = self.anim_actions_menu[i];
            let anim_y = (anim * 80.0).round() as i32;
            let x_offset = (i * 160) as i32;

            if let Some((start, damage)) = self.anim_party_damage[i] {
                let anim = 0.5_f32.powf((time - start) as f32 * 4.0);
                if damage == 0 {
                    d.draw_texture(
                        sprite,
                        20 + (rng.random_range(-4.0..4.0) * anim).round() as i32 + x_offset,
                        480 - 128 - anim_y + (f64::sin(time * 2.0) * 4.0).round() as i32,
                        Color::new(255, 255, 255, 127),
                    );

                    d.draw_text_ex(
                        font,
                        "MISS",
                        Vector2::new(50.0 + x_offset as f32, 480.0 - 128.0 + anim * 40.0),
                        16.0,
                        1.0,
                        Color::new(255, 255, 255, 127),
                    );
                } else {
                    d.draw_texture(
                        sprite,
                        20 + (rng.random_range(-4.0..4.0) * anim).round() as i32 + x_offset,
                        480 - 128 + (f64::sin(time * 2.0) * 4.0).round() as i32 - anim_y
                            + (rng.random_range(-4.0..4.0) * anim).round() as i32,
                        Color::color_from_hsv(0.0, anim * 0.5, 1.0),
                    );

                    d.draw_text_ex(
                        font,
                        &format!("-{damage}"),
                        Vector2::new(50.0 + x_offset as f32, 480.0 - 128.0 + anim * 40.0),
                        16.0,
                        1.0,
                        Color::RED,
                    );
                }
            } else {
                d.draw_texture(
                    sprite,
                    20 + x_offset,
                    480 - 128 + (f64::sin(time * 2.0) * 4.0).round() as i32 - anim_y,
                    Color::WHITE,
                );
            }

            d.draw_rectangle(x_offset, 480 - anim_y, 160, 80, Color::BLACK);

            d.draw_text_ex(
                font,
                &member.info().name.to_uppercase(),
                Vector2::new(5.0 + x_offset as f32, 463.0 - anim_y as f32),
                16.0,
                1.0,
                Color::WHITE,
            );

            let health_display = format!("{} / {}", member.health(), member.info().health);
            d.draw_text_ex(
                font,
                &health_display,
                Vector2::new(
                    160.0 - font.measure_text(&health_display, 16.0, 1.0).x + x_offset as f32,
                    463.0 - anim_y as f32,
                ),
                16.0,
                1.0,
                Color::WHITE,
            );

            let action_buttons = res.tex("battle_actions");

            for j in 0..4 {
                d.draw_texture_rec(
                    action_buttons,
                    Rectangle::new(0.0, 15.0 * j as f32, 100.0, 15.0),
                    Vector2::new(
                        30.0 + x_offset as f32,
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
            attack.draw(d, res, time, frame_count, font, rng);
        }
    }
}
