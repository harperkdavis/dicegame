mod long;
mod music;
mod short;

use std::fs;

pub use long::Long;
use rand::Rng;
use raylib::{
    color::Color,
    prelude::{RaylibDraw, RaylibDrawHandle},
};
pub use short::Short;

use crate::{
    game::{Frame, INPUT_CONFIRM, Static, content::room::Room, state::short::ActiveSequence},
    interface::BattleInterface,
};

pub const TICK_RATE: usize = 50;
pub const TICK_TIME: f32 = 1.0 / TICK_RATE as f32;

pub struct State<'a> {
    pub long: Long,
    pub short: Short<'a>,
}

fn load<'a>(savefile: &str, s: Static<'a>) -> eyre::Result<(Long, Short<'a>)> {
    let long = toml::from_str(savefile)?;
    let short = Short::from_long(&long, s)?;

    Ok((long, short))
}

pub fn load_file<'a>(s: Static<'a>) -> eyre::Result<(Long, Short<'a>)> {
    let file_contents = fs::read_to_string("save.toml");
    match file_contents {
        Ok(savefile) => load(&savefile, s),
        Err(_) => {
            let long = Long::default();
            fs::write("save.toml", toml::to_string_pretty(&long)?)?;
            let short = Short::from_long(&long, s)?;
            Ok((long, short))
        }
    }
}

const PLAYER_SPEED: f32 = 100.0;
const PLAYER_W: f32 = 16.0;
const PLAYER_H: f32 = 8.0;

// returns true if encounter
fn update_movement(
    room: &Room,
    short: &mut Short,
    frame: Frame,
    rng: &mut impl Rng,
) -> eyre::Result<bool> {
    let new_x = short.pos_x + frame.input_x * PLAYER_SPEED * frame.delta;
    let new_y = short.pos_y + frame.input_y * PLAYER_SPEED * frame.delta;

    if (new_x - short.pos_x).abs() + (new_y - short.pos_y).abs() > 0.001
        && room
            .enemy_chance
            .is_some_and(|c| rng.random_bool(c * frame.delta as f64))
    {
        return Ok(true);
    }

    short.pos_x = new_x;
    short.pos_y = new_y;

    if frame.actions_down[INPUT_CONFIRM] {
        for t in &room.layout.triggers {
            if t.r.is_within(short.pos_x as i32, short.pos_y as i32) {
                short.seq = Some(ActiveSequence::new(
                    &room.room,
                    t.seq.clone(),
                    &short.lua,
                    frame.time,
                )?);
                break;
            }
        }
    }

    Ok(false)
}

pub fn update(
    d: &RaylibDrawHandle,
    State { long, short }: &mut State,
    s: Static,
    frame: Frame,
) -> eyre::Result<()> {
    let Frame { time, .. } = frame;

    let room = s.room(&long.room);
    let mut rng = rand::rng();

    let reset = if let Some(bi) = short.battle.as_mut() {
        bi.update(d, s, frame, &mut rng)
    } else {
        if let Some(seq) = short.seq.as_mut() {
            if seq.update(long, frame)? {
                short.seq = None;
            }
        } else {
            if let Some(e_time) = short.enemy_encounter {
                if time - e_time > 1.45 {
                    short.enemy_encounter = None;
                    short.battle = Some(BattleInterface::new(time, s));
                }
            } else if update_movement(room, short, frame, &mut rng)? {
                s.snd("enemy_encounter").play();
                short.enemy_encounter = Some(time);
            };
        }
        false
    };

    short.music.update(
        d.get_frame_time(),
        short.enemy_encounter.is_some(),
        short
            .battle
            .as_ref()
            .map(|a| (a.battle_result().map(|a| a.0), a.music_volume(time))),
    );

    if reset {
        short.battle = None;
        short.battle_result = None;
    }

    Ok(())
}

pub fn draw(
    d: &mut impl RaylibDraw,
    State { long, short }: &mut State,
    s: Static,
    frame: Frame,
) -> eyre::Result<()> {
    let mut rng = rand::rng();
    let room = s.room(&long.room);

    if let Some(bi) = short.battle.as_mut() {
        bi.draw(d, s, frame, &mut rng);
    } else {
        room.draw_background(d, s);
        room.draw(d, s, None);
        d.draw_rectangle(
            (short.pos_x - PLAYER_W / 2.0) as i32,
            (short.pos_y - PLAYER_H / 2.0) as i32,
            16,
            8,
            Color::BLUE,
        );

        if let Some(seq) = short.seq.as_mut() {
            seq.draw(d, s, frame);
        }

        if let Some(et) = short.enemy_encounter {
            let anim_out = ((frame.time - et) / 1.5).powi(20);
            d.draw_rectangle(
                0,
                0,
                640,
                480,
                Color::new(255, 255, 255, (anim_out * 255.0).min(255.0) as u8),
            );
        }
    }

    Ok(())
}
