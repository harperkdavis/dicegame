use std::{array, iter};

use rand::{
    Rng,
    seq::{IndexedRandom, IteratorRandom},
};
use raylib::prelude::*;

use crate::{
    dice::DEFAULT_SET,
    game::{
        Frame, INPUT_CANCEL, INPUT_CONFIRM, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_UP, Static,
        battle::{
            Action, Battle, DamageEvent, Rewards,
            health::{HEAD_INDEX, LIMBS, MAX_HEAD_HEALTH, MAX_HEALTH_VALUES},
            text,
        },
    },
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
    rewards: Option<Rewards>,
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
    anim_action_wait: Option<f64>,
    anim_party_damage_queue: Vec<DamageEvent>,

    anim_event_text: [Option<(String, f64)>; MAX_TEXT_COUNT],

    anim_reward_reveal: Option<Option<usize>>,
    anim_reward_money: u32,
    anim_reward_money_prev: u32,
    anim_reward_money_digits: [f64; 5],

    anim_fadeout: Option<f64>,
}

const ENEMY_X: i32 = 450;
const ENEMY_Y: i32 = 200;
const TWO_BAR_DURATION: f64 = 60.0 / 160.0 * 7.0;
const MAX_TEXT_COUNT: usize = 6;
const TEXT_READ_TIME: f64 = 3.0;

impl BattleInterface {
    pub fn new(time: f64, s: Static) -> Self {
        let battle = Battle::versus(
            &[
                s.party("enn"),
                s.party("ess"),
                s.party("elle"),
                s.party("mensh"),
            ],
            s.enemy("fleshthing"),
        );
        let party_count = battle.party().len();
        let enemy_count = battle.enemies().len();

        Self {
            battle,
            rewards: None,
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
            anim_action_wait: None,
            anim_party_damage_queue: Vec::new(),

            anim_event_text: array::from_fn(|_| None),

            anim_reward_reveal: None,
            anim_reward_money: 0,
            anim_reward_money_prev: 0,
            anim_reward_money_digits: [0.0; 5],

            anim_fadeout: None,
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
        self.anim_action_wait = None;
        self.anim_party_damage_queue = Vec::new();
    }

    pub fn reset_enemy_animation(&mut self) {
        let enemy_count = self.battle.party().len();
        self.anim_enemy_damage = iter::repeat_n(None, enemy_count).collect();
    }

    fn push_event_text(&mut self, text: String, time: f64) -> usize {
        let index = match self.anim_event_text.iter().position(Option::is_none) {
            Some(u) => u,
            None => {
                self.anim_event_text
                    .iter()
                    .enumerate()
                    .min_by(|(_, oa), (_, ob)| {
                        f64::total_cmp(&oa.as_ref().unwrap().1, &ob.as_ref().unwrap().1)
                    })
                    .unwrap()
                    .0
            }
        };

        self.anim_event_text[index] = Some((text, time));

        index
    }

    pub fn music_volume(&self, time: f64) -> f32 {
        if let Some(start) = self.anim_fadeout {
            let elapsed = time - start;
            (1.0 - elapsed as f32).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    pub fn update(
        &mut self,
        d: &RaylibDrawHandle,
        s: Static,
        frame: Frame,
        rng: &mut impl Rng,
    ) -> bool {
        let time = frame.time;

        if let Some(anim_fadeout) = self.anim_fadeout
            && time - anim_fadeout >= 1.0
        {
            return true;
        }
        if let Some(end) = self.battle_end {
            let elapsed = time - end;
            let rewards = self.rewards.as_ref().unwrap();

            if elapsed > 6.0 {
                match self.anim_reward_reveal {
                    // Yet to reveal
                    None => {
                        // if we want a sound effect or something it goes here.
                        self.anim_reward_reveal = Some(None);
                    }
                    // Revealing money (2s)
                    Some(None) => {
                        let anim_in = 1.0 - (1.0 - (elapsed - 6.0) / 2.0).powi(3).clamp(0.0, 1.0);
                        self.anim_reward_money =
                            (anim_in * rewards.money as f64 / 5.0).floor() as u32 * 5;

                        if self.anim_reward_money > self.anim_reward_money_prev {
                            s.snd("reward_money").set_pitch((0.25 + anim_in) as f32);
                            s.snd("reward_money").play();

                            let before = format!("{:05}", self.anim_reward_money_prev);
                            let after = format!("{:05}", self.anim_reward_money);

                            for i in 0..5 {
                                if before.chars().nth(i) != after.chars().nth(i) {
                                    self.anim_reward_money_digits[i] = time;
                                }
                            }
                            self.anim_reward_money_prev = self.anim_reward_money;
                        }

                        if elapsed >= 8.0 {
                            self.anim_reward_money_digits = [time; 5];
                            self.anim_reward_reveal = Some(Some(0));

                            s.snd("reward_money_final").play();
                        }
                    }
                    // Revealing items (0.5s per)
                    Some(Some(reveal)) => {
                        self.anim_reward_money = rewards.money;
                        self.anim_reward_money_prev = rewards.money;

                        let current_reveal =
                            (((elapsed - 8.0) / 0.5).floor() as usize).min(rewards.items.len());
                        if current_reveal > reveal {
                            self.anim_reward_reveal = Some(Some(current_reveal));

                            s.snd("reward_item").play();
                        }
                    }
                }
            }

            if frame.actions_down[INPUT_CONFIRM] {
                match self.anim_reward_reveal {
                    Some(Some(i)) if i == rewards.items.len() => {
                        if self.anim_fadeout.is_none() {
                            self.anim_fadeout = Some(time);
                        }
                    }
                    _ => {
                        s.snd("reward_money_final").play();
                        if !rewards.items.is_empty() {
                            s.snd("reward_item").play();
                        }
                        self.anim_reward_reveal = Some(Some(rewards.items.len()))
                    }
                }
            }
            return false;
        }

        if self.in_intro {
            if time - self.battle_start >= TWO_BAR_DURATION || frame.actions_down[INPUT_CONFIRM] {
                self.in_intro = false;
                self.set_menu_animation_state();
                // battle start needs to be when the music starts or offset by the two bar duration
                if !frame.actions_down[INPUT_CONFIRM] {
                    self.battle_start += TWO_BAR_DURATION;
                }

                let intro_flavors = self
                    .battle
                    .enemies()
                    .iter()
                    .filter_map(|e| e.info().generate_flavor_intro(rng))
                    .collect::<Box<[_]>>();

                for f in intro_flavors {
                    self.push_event_text(f.to_owned(), time);
                }
            }
            return false;
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

        for i in 0..MAX_TEXT_COUNT {
            if let Some((_, start)) = self.anim_event_text[i].as_ref()
                && time - start >= TEXT_READ_TIME
            {
                self.anim_event_text[i] = None;
            }
        }

        if self.battle.is_player_turn() {
            if let Some(enemy) = self.enemy_select.as_mut() {
                let max = self.battle.enemies().len() - 1;
                if frame.actions_down[INPUT_LEFT] || frame.actions_down[INPUT_UP] {
                    if *enemy == 0 {
                        *enemy = max;
                    } else {
                        *enemy -= 1;
                    }
                }
                if frame.actions_down[INPUT_RIGHT] || frame.actions_down[INPUT_DOWN] {
                    if *enemy == max {
                        *enemy = 0;
                    } else {
                        *enemy += 1;
                    }
                }
                if frame.actions_down[INPUT_CANCEL] {
                    self.enemy_select = None;
                    s.snd("menu").play();
                } else if frame.actions_down[INPUT_CONFIRM] {
                    self.battle.push_action(Action::Attack(*enemy));
                    self.anim_actions_hover = [0.0; 4];
                    s.snd("select").play();
                    self.enemy_select = None;
                }
            } else {
                if frame.actions_down[INPUT_UP] {
                    s.snd("menu").play();
                    if self.action_select == 0 {
                        self.action_select = 2;
                    } else {
                        self.action_select -= 1;
                    }
                }
                if frame.actions_down[INPUT_DOWN] {
                    s.snd("menu").play();
                    if self.action_select == 2 {
                        self.action_select = 0;
                    } else {
                        self.action_select += 1;
                    }
                }
                if frame.actions_down[INPUT_CONFIRM] {
                    if self.action_select == 0 {
                        s.snd("select").play();
                        self.enemy_select = Some(0);
                    } else if self.action_select == 1 {
                        s.snd("select").play();
                        self.battle.push_action(Action::Defend);
                    }
                }
            }
        } else if self.is_enemy_turn {
            // enemy turn
            if time > self.anim_last_damage + 1.0 || frame.actions_down[INPUT_CONFIRM] {
                self.anim_last_damage = time;
                if let Some(DamageEvent { to, amount, .. }) = self.anim_party_damage_queue.pop() {
                    s.snd("player_hurt").set_pitch(rng.random_range(0.9..1.1));
                    s.snd("player_hurt").play();
                    self.anim_party_damage[to] = Some((time, amount));
                } else {
                    self.is_enemy_turn = false;
                    self.battle.finish_enemy_turn();
                    if self.battle.battle_result().is_some() {
                        self.rewards = Some(
                            self.battle
                                .rewards()
                                .expect("should get rewards (battle is finished)"),
                        );
                        // consider adding back but its a little annoying
                        // let music_progress = time - self.battle_start;
                        // let next_end_time = (music_progress / TWO_BAR_DURATION).ceil()
                        //    * TWO_BAR_DURATION
                        //    + self.battle_start;
                        self.battle_end = Some(time);
                    } else {
                        let flavor_options = self
                            .battle
                            .enemies()
                            .iter()
                            .filter_map(|e| {
                                (e.health() > 0)
                                    .then(|| e.info().generate_flavor(rng))
                                    .flatten()
                            })
                            .collect::<Box<[_]>>();
                        if let Some(flavor) = flavor_options.choose(rng) {
                            self.push_event_text(flavor.to_string(), time);
                        }

                        self.set_menu_animation_state();
                    }
                }
            }
        } else if let Some((action, party_member)) = &self.current_action {
            let member = &self.battle.party()[*party_member];
            if let Some(expires) = self.anim_action_wait {
                if time >= expires || frame.actions_down[INPUT_CONFIRM] {
                    self.anim_action_wait = None;
                    self.attack = None;
                    self.attack_result = None;
                    self.current_action = None;
                }
            } else {
                match action {
                    Action::Attack(target) => {
                        if let Some(damage) = &self.attack_result {
                            let attack = self.attack.as_ref().unwrap();
                            if let Some(dat) = attack.damage_apply_time()
                                && time > dat
                                && self.anim_enemy_damage[*target].is_none()
                            {
                                let enemy_def = self.battle.enemies()[*target].info();

                                if *damage == 0 {
                                    s.snd("attack_miss").set_pitch(rng.random_range(0.9..1.1));
                                    s.snd("attack_miss").play();
                                } else {
                                    s.snd("enemy_hurt").set_pitch(rng.random_range(0.9..1.1));
                                    s.snd("enemy_hurt").play();
                                }

                                self.anim_enemy_damage[*target] = Some((dat, *damage));
                                if self.battle.enemies()[*target].health() == 0 {
                                    // TODO: kill sound
                                    self.anim_enemy_death[*target] = Some(dat);
                                    s.snd("enemy_hurt").stop();
                                    s.snd("enemy_death").play();

                                    if let Some(flavor) = enemy_def.generate_flavor_defeat(rng) {
                                        self.push_event_text(flavor.to_owned(), time);
                                    }
                                }
                            }

                            let attack = self.attack.as_ref().unwrap();
                            // after
                            if attack.can_advance(time, frame.actions_down[INPUT_CONFIRM]) {
                                self.attack = None;
                                self.attack_result = None;
                                self.current_action = None;
                            }
                        } else if let Some(attack) = &mut self.attack {
                            if let Some(damage) = attack.update(d, s, rng, time) {
                                self.attack_result = Some(damage);
                                self.battle.apply_damage(*target, damage, rng);
                            }
                        } else {
                            self.attack = Some(AttackInterface::new_round(
                                s,
                                DEFAULT_SET,
                                rng,
                                *member.health(),
                                time,
                                Vector2::new(ENEMY_X as f32 - 10.0, ENEMY_Y as f32 - 30.0),
                            ));
                        }
                    }
                    Action::Defend => {
                        let text = text::replace_placeholders(
                            text::DEFEND_LINES.choose(rng).unwrap(),
                            member.info(),
                        );

                        s.snd("action_defend").set_volume(0.5);
                        s.snd("action_defend").play();
                        self.push_event_text(text, time);
                        self.anim_action_wait = Some(time + 1.0);
                    }
                    Action::Flee => (),
                }
            }
        } else if let Some(next_action) = self.battle.process_next_action() {
            self.current_action = Some(next_action);
            self.reset_enemy_animation();
        } else {
            self.current_action = None;
            self.is_enemy_turn = true;
            self.anim_party_damage_queue = self.battle.run_enemy_turn(rng);
        }

        false
    }

    pub fn battle_result(&self) -> Option<(bool, f64)> {
        self.battle_end
            .and_then(|a| self.battle.battle_result().map(|b| (b, a)))
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        s: Static,
        Frame {
            time, frame_count, ..
        }: Frame,
        rng: &mut impl Rng,
    ) {
        let mut shader = s.sha("battle_background").borrow_mut();
        let battle_background_ocean = s.tex("battle_background_ocean");

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
                    3.0,
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
                s.tex("enn_battleintro"),
                -70 - (anim_in * 20.0) as i32,
                150 + (anim_in.powi(2) * 400.0) as i32,
                Color::WHITE,
            );
            d.draw_texture(
                s.tex("fleshthing_concept"),
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

        let text_font = s.fnt("nokia_15");
        let numbers_font = s.fnt("execution");
        // draw actual battle
        for (i, enemy) in self.battle.enemies().iter().enumerate() {
            let sprite = s.tex(enemy.info().sprite.as_str());
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
                d.draw_text_outline(
                    text_font,
                    &enemy.info().name.to_uppercase(),
                    ENEMY_X as f32,
                    ENEMY_Y as f32,
                    Color::WHITE,
                    Color::BLACK,
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
            s.tex("girl2"),
            200 - (anim_in_battle * 200.0) as i32,
            300,
            Color::WHITE,
        );

        for (i, member) in self.battle.party().iter().enumerate() {
            let sprite = s.tex(member.info().sprite_battle.as_str());
            let anim = self.anim_actions_menu[i];
            let anim_y = (anim * 80.0).round() as i32 - (anim_in_battle * 200.0) as i32;
            let x_offset = (i * 160) as i32;
            let char_y = 470 - 128 - anim_y
                + (f64::sin(time * 2.0 + i as f64) * 4.0).round() as i32
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
                s.tex("ui_charactername"),
                x_offset,
                460 - anim_y,
                Color::new(51, 51, 51, 255),
            );

            d.draw_text_outline(
                text_font,
                &member.info().name,
                4.0 + x_offset as f32,
                444.0 - anim_y as f32,
                Color::WHITE,
                Color::BLACK,
            );

            d.draw_texture(
                s.tex("ui/body/default/base"),
                x_offset + 115,
                460 - anim_y,
                Color::WHITE,
            );

            for (i, seg) in LIMBS.iter().enumerate() {
                let health = member.health()[i];
                let max_health = MAX_HEALTH_VALUES[i];
                let limb_color = super::health_color(health, max_health, time);
                d.draw_texture(
                    s.tex(format!("ui/body/default/{seg}").as_str()),
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
                s.fnt("execution_small"),
                "/400",
                x_offset as f32 + 35.0,
                463.0 - anim_y as f32,
                Color::WHITE,
                Color::BLACK,
            );

            d.draw_rectangle(x_offset, 480 - anim_y, 160, 80, Color::BLACK);

            let action_buttons = s.tex("battle_actions");

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
            attack.draw(d, s, time, frame_count, numbers_font, rng);
        }

        for i in 0..MAX_TEXT_COUNT {
            if let Some((text, start)) = self.anim_event_text[i].as_ref() {
                let elapsed = time - start;
                let fade_in = (elapsed * 4.0).min(1.0) as f32;
                let anim_in = f32::exp((-elapsed * 4.0) as f32);
                let anim_out = (1.0 - (TEXT_READ_TIME - elapsed).min(1.0)).powi(5).min(1.0) as f32;
                d.draw_text_outline(
                    s.fnt("nokia_15"),
                    text,
                    200.0 + (i * 10) as f32 - anim_out * 500.0 - elapsed as f32 * 20.0,
                    120.0 + anim_in * 10.0 + (i * 20) as f32,
                    Color::WHITE.alpha(fade_in),
                    Color::BLANK.alpha(fade_in),
                );
            }
        }

        if let Some(end_time) = self.battle_end {
            let reveal_start = end_time + (7.0 / 8.0);
            let reveal_count = if self.anim_reward_reveal.is_some() {
                9
            } else {
                ((time - reveal_start).max(0.0) * 8.0).ceil() as usize
            };

            let elapsed = time - end_time;
            let full_anim = if elapsed > 2.0 {
                0.5_f64.powf((elapsed - 2.0) * 2.0)
            } else {
                0.0
            };

            let box_anim_in = if self.anim_reward_reveal.is_some() {
                1.0
            } else {
                ((elapsed - 4.0).max(0.0) / 2.0).min(1.0).powi(21) as f32
            };

            d.draw_rectangle(
                0,
                (90.0 - box_anim_in * 50.0) as i32,
                640,
                (200.0 * box_anim_in) as i32,
                Color::BLACK,
            );

            for i in 0..reveal_count.min(9) {
                let revealed_at = reveal_start + i as f64 / 8.0;
                let letter_elapsed = time - revealed_at;
                let letter_anim = if self.anim_reward_reveal.is_some() {
                    0.0
                } else {
                    0.5_f64.powf(letter_elapsed * 8.0) as f32
                };
                d.draw_texture_rec(
                    s.tex("battle_result_letters"),
                    Rectangle::new(i as f32 * 22.0, 0.0, 22.0, 42.0),
                    Vector2::new(
                        110.0 + 50.0 * i as f32 + rng.random_range(-4.0..=4.0) * full_anim as f32,
                        65.0 + letter_anim * 10.0
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

            if elapsed > 6.0 || self.anim_reward_reveal.is_some() {
                let rewards = self.rewards.as_ref().unwrap();

                if let Some(o) = self.anim_reward_reveal.as_ref() {
                    d.draw_text_ex(
                        text_font,
                        "REWARD$",
                        Vector2::new(110.0, 140.0),
                        15.0,
                        0.0,
                        Color::WHITE,
                    );

                    let reward_money = if o.is_some() {
                        format!("{: >5}", rewards.money)
                    } else {
                        format!("{: >5}", self.anim_reward_money)
                    };
                    for i in 0..5 {
                        if let Some(c) = reward_money
                            .chars()
                            .nth(i)
                            .and_then(|c| if c.is_whitespace() { None } else { Some(c) })
                        {
                            let anim =
                                f64::exp(-(time - self.anim_reward_money_digits[i]).max(0.0) * 8.0);
                            d.draw_text_ex(
                                numbers_font,
                                &format!("{c}"),
                                Vector2::new(190.0 + i as f32 * 10.0, 142.0 - anim as f32 * 3.0),
                                16.0,
                                0.0,
                                Color::WHITE,
                            );
                        }
                    }

                    if let Some(ind) = o {
                        d.draw_text_ex(
                            text_font,
                            "ITEMS",
                            Vector2::new(320.0, 140.0),
                            15.0,
                            0.0,
                            Color::WHITE,
                        );

                        for (i, (item, count)) in rewards.items.iter().enumerate() {
                            if *ind > i {
                                let name = item;
                                d.draw_text_ex(
                                    text_font,
                                    &format!("{name} x{count}"),
                                    Vector2::new(400.0, 140.0 + i as f32 * 20.0),
                                    15.0,
                                    0.0,
                                    Color::WHITE,
                                );
                            }
                        }
                    }
                }
            }
        }

        if let Some(anim_fadeout) = self.anim_fadeout {
            d.draw_rectangle(
                0,
                0,
                640,
                480,
                Color::BLACK.alpha((time - anim_fadeout) as f32),
            );
        }
    }
}
