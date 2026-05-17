use std::{collections::HashSet, fs};

use rand::Rng;
use raylib::{
    audio::{Music, RaylibAudio},
    color::Color,
    ffi::KeyboardKey,
    prelude::{RaylibDraw, RaylibDrawHandle, RaylibMode2D},
};
use serde::{Deserialize, Serialize};

use crate::{
    game::{
        Str,
        content::{Cnt, room::Room},
    },
    interface::BattleInterface,
    res::Res,
};

pub const TICK_RATE: usize = 50;
pub const TICK_TIME: f32 = 1.0 / TICK_RATE as f32;

pub struct PartyMember {
    def: Str,
    health: u32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Long {
    flags: HashSet<Str>,
    room: Str,
}

pub fn load<'a>(
    savefile: &str,
    res: &Res,
    cnt: Cnt,
    ra: &'a RaylibAudio,
) -> eyre::Result<(Long, Short<'a>)> {
    let long = toml::from_str(savefile)?;
    let short = Short::from_long(&long, res, cnt, ra)?;

    Ok((long, short))
}

pub fn load_file<'a>(res: &Res, cnt: Cnt, ra: &'a RaylibAudio) -> eyre::Result<(Long, Short<'a>)> {
    let file_contents = fs::read_to_string("save.toml");
    match file_contents {
        Ok(savefile) => load(&savefile, res, cnt, ra),
        Err(_) => {
            let long = Long::default();
            fs::write("save.toml", toml::to_string_pretty(&long)?)?;
            let short = Short::from_long(&long, res, cnt, ra)?;
            Ok((long, short))
        }
    }
}

pub struct Short<'a> {
    pos_x: i32,
    pos_y: i32,
    background_music: Music<'a>,
    battle_music: Music<'a>,
    victory_music: Music<'a>,

    enemy_encounter: Option<f64>,
    battle: Option<BattleInterface>,
    battle_result: Option<bool>,
}

impl<'a> Short<'a> {
    pub fn from_long(long: &Long, res: &Res, cnt: Cnt, ra: &'a RaylibAudio) -> eyre::Result<Self> {
        let room = &cnt.rooms[&long.room];
        let mut background_music = res.load_mus(room.music.as_str(), ra);
        background_music.set_pitch(room.music_pitch.unwrap_or(1.0) as f32);
        background_music.set_volume(0.2);
        background_music.play_stream();
        background_music.looping = true;

        let mut battle_music = res.load_mus("003_EVISCERATE!", ra);
        battle_music.set_volume(0.2);
        battle_music.looping = true;

        let mut victory_music = res.load_mus("005_EXECUTION!", ra);
        victory_music.set_volume(0.5);
        victory_music.looping = false;

        Ok(Self {
            pos_x: 0,
            pos_y: 0,
            background_music,
            battle_music,
            victory_music,

            enemy_encounter: None,
            battle: None,
            battle_result: None,
        })
    }
}

const PLAYER_SPEED: i32 = 2;
const PLAYER_W: i32 = 16;
const PLAYER_H: i32 = 8;

// returns true if encounter
fn update_movement(
    d: &RaylibDrawHandle,
    room: &Room,
    short: &mut Short,
    rng: &mut impl Rng,
) -> bool {
    let mut input_x = 0;
    let mut input_y = 0;

    if d.is_key_down(KeyboardKey::KEY_RIGHT) {
        input_x += 1;
    }
    if d.is_key_down(KeyboardKey::KEY_LEFT) {
        input_x -= 1;
    }

    if d.is_key_down(KeyboardKey::KEY_UP) {
        input_y -= 1;
    }
    if d.is_key_down(KeyboardKey::KEY_DOWN) {
        input_y += 1;
    }

    let new_x = short.pos_x + input_x * PLAYER_SPEED;
    let new_y = short.pos_y + input_y * PLAYER_SPEED;

    if (new_x - short.pos_x).abs() + (new_y - short.pos_y).abs() > 0
        && room
            .enemy_chance
            .is_some_and(|c| rng.random_bool(c / TICK_RATE as f64))
    {
        return true;
    }

    short.pos_x = new_x;
    short.pos_y = new_y;

    false
}

// fixed timestep
pub fn tick(d: &RaylibDrawHandle, long: &mut Long, short: &mut Short, res: &Res, cnt: Cnt) {
    let mut rng = rand::rng();
    let time = d.get_time();
    let room = &cnt.rooms[&long.room];

    if short.enemy_encounter.is_some() || short.battle.is_some() {
        short.background_music.set_volume(0.0);
    }

    if let Some(_bi) = short.battle.as_mut() {
        short.battle_music.update_stream();
    } else if let Some(e_time) = short.enemy_encounter {
        if time - e_time > 1.45 {
            short.enemy_encounter = None;
            short.battle = Some(BattleInterface::new(time, cnt));
            short.battle_music.seek_stream(0.0);
            short.battle_music.play_stream();
        }
    } else if update_movement(d, room, short, &mut rng) {
        res.snd("enemy_encounter").play();
        short.enemy_encounter = Some(time);
    }
}

pub fn update(d: &RaylibDrawHandle, _long: &mut Long, short: &mut Short, res: &Res, _cnt: Cnt) {
    let mut rng = rand::rng();
    let time = d.get_time();
    if let Some(bi) = short.battle.as_mut() {
        bi.update(d, res, &mut rng, time);
        if short.battle_result.is_some() {
            short.victory_music.update_stream();
        } else {
            short.background_music.update_stream();

            if let Some((win, change_music_at)) = bi.battle_result()
                && time >= change_music_at
            {
                short.battle_result = Some(win);
                short.battle_music.stop_stream();
                short.victory_music.seek_stream(0.0);
                short.victory_music.play_stream();
            }
        }
    }
}

pub fn draw(
    d: &mut impl RaylibDraw,
    long: &Long,
    short: &mut Short,
    res: &Res,
    cnt: Cnt,
    time: f64,
    frame_count: usize,
) {
    let mut rng = rand::rng();
    let room = &cnt.rooms[&long.room];

    if let Some(bi) = short.battle.as_mut() {
        bi.draw(d, res, time, frame_count, &mut rng);
    } else {
        d.draw_rectangle(
            short.pos_x - PLAYER_W / 2,
            short.pos_y - PLAYER_H / 2,
            PLAYER_W,
            PLAYER_H,
            Color::BLUE,
        );

        if let Some(et) = short.enemy_encounter {
            let anim_out = ((time - et) / 1.5).powi(20);
            d.draw_rectangle(
                0,
                0,
                640,
                480,
                Color::new(255, 255, 255, (anim_out * 255.0).min(255.0) as u8),
            );
        }
    }
}
