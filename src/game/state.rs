pub mod long;
mod music;
mod short;

use std::{collections::HashMap, fs};

pub use long::Long;
use rand::Rng;
use raylib::{
    camera::Camera2D,
    color::Color,
    math::{Rectangle, Vector2},
    prelude::{RaylibDraw, RaylibDrawHandle, RaylibMode2DExt},
};
pub use short::Short;

use crate::{
    Str,
    game::{
        Frame, INPUT_CONFIRM, Static,
        content::room::{PropsExt, Room},
        state::{long::Flags, short::ActiveSequence},
    },
    interface::BattleInterface,
    util::Direction,
};

pub const TICK_RATE: usize = 50;
pub const TICK_TIME: f32 = 1.0 / TICK_RATE as f32;
pub const TRANSITION_TIME: f64 = 0.25;

pub struct State<'a> {
    pub long: Long,
    pub short: Short<'a>,
}

fn load<'a>(savefile: &str, s: Static<'a>) -> eyre::Result<(Long, Short<'a>)> {
    let long = toml_edit::de::from_str(savefile)?;
    let short = Short::from_long(&long, s, None)?;

    Ok((long, short))
}

pub fn load_file<'a>(s: Static<'a>) -> eyre::Result<(Long, Short<'a>)> {
    let file_contents = fs::read_to_string("save.toml");
    match file_contents {
        Ok(savefile) => load(&savefile, s),
        Err(_) => {
            let long = Long::default();
            fs::write("save.toml", toml_edit::ser::to_string_pretty(&long)?)?;
            let short = Short::from_long(&long, s, None)?;
            Ok((long, short))
        }
    }
}

pub fn load_playtest<'a>(room: Room, s: Static<'a>) -> eyre::Result<(Long, Short<'a>)> {
    let long = Long::default_with_room(room.room.as_str().into());
    let short = Short::from_long(&long, s, Some(room))?;
    Ok((long, short))
}

const PLAYER_SPEED: f32 = 140.0;
const PLAYER_W: f32 = 20.0;
const PLAYER_H: f32 = 16.0;
const LOOK_MULTIPLIER: f32 = 2.0;

pub fn get_room<'a>(room: &'a Str, short: &'a Short, s: Static<'a>) -> &'a Room {
    if let Some(ro) = &short.room_override
        && &ro.room == room
    {
        ro
    } else {
        s.room(room)
    }
}

// returns true if encounter
fn update_movement(
    room: &Room,
    short: &mut Short,
    flags: &Flags,
    frame: Frame,
    rng: &mut impl Rng,
) -> eyre::Result<bool> {
    if short.room_transition_to.is_some() {
        short.walk = false;
        return Ok(false);
    }

    let room = if let Some(ro) = &short.room_override
        && room.room == ro.room
    {
        // use override
        ro
    } else {
        room
    };

    let mov = Vector2::new(frame.input_x, frame.input_y).normalized();

    if mov.y > 0.5 {
        short.dir = Direction::South;
    } else if mov.y < -0.5 {
        short.dir = Direction::North;
    } else if mov.x > 0.5 {
        short.dir = Direction::East;
    } else if mov.x < -0.5 {
        short.dir = Direction::West;
    }

    let mut new_x = short.pos_x + mov.x * PLAYER_SPEED * frame.delta;
    let mut new_y = short.pos_y + mov.y * PLAYER_SPEED * frame.delta;

    const EPSILON: f32 = 1e-5;

    for c in &room.layout.collision {
        if !c.props.is_enabled(flags) {
            continue;
        }
        let r = c.r;
        let (nlx, nrx, nty, nby) = (
            new_x - PLAYER_W / 2.0,
            new_x + PLAYER_W / 2.0,
            new_y - PLAYER_H / 2.0,
            new_y + PLAYER_H / 2.0,
        );
        let (rlx, rrx, rty, rby) = (
            r.x as f32,
            r.right_x() as f32,
            r.y as f32,
            r.bottom_y() as f32,
        );
        if nlx < rrx && nrx > rlx && nty < rby && nby > rty {
            let depth_x = f32::abs(nrx.min(rrx) - nlx.max(rlx));
            let depth_y = f32::abs(nby.min(rby) - nty.max(rty));

            if depth_x < depth_y {
                if nrx < rrx {
                    new_x -= depth_x + EPSILON;
                } else if nlx > rlx {
                    new_x += depth_x + EPSILON;
                }
            } else {
                if nty > rty {
                    new_y += depth_y + EPSILON;
                } else if nby < rby {
                    new_y -= depth_y + EPSILON;
                }
            }
        }
    }

    let moved = (new_x - short.pos_x).abs() + (new_y - short.pos_y).abs() > 0.001;

    short.pos_x = new_x;
    short.pos_y = new_y;

    for n in &room.layout.transitions {
        if !n.props.is_enabled(flags) {
            continue;
        }
        if let Some(to) = &n.to_room
            && n.r.is_within_f(new_x, new_y)
        {
            short.room_transition_to = Some((
                frame.time + TRANSITION_TIME,
                to.clone(),
                n.to_transition.clone(),
            ));
        }
    }

    if moved {
        short.walk = true;
        if room
            .enemy_chance
            .is_some_and(|c| rng.random_bool(c * frame.delta as f64))
        {
            return Ok(true);
        }
    } else {
        short.walk = false;
    }

    if frame.actions_down[INPUT_CONFIRM] {
        let (look_x, look_y) = short.dir.to_vec();
        let (look_x, look_y) = (
            look_x as f32 * PLAYER_W / 2.0 * LOOK_MULTIPLIER + short.pos_x,
            look_y as f32 * PLAYER_H / 2.0 * LOOK_MULTIPLIER + short.pos_y,
        );
        for t in &room.layout.triggers {
            if !t.props.is_enabled(flags) {
                continue;
            }
            if (t.inner.is_some_and(|b| b) && t.r.is_within_f(short.pos_x, short.pos_y))
                || (t.inner.is_none_or(|b| !b) && t.r.is_within_f(look_x, look_y))
            {
                short.seq = Some(ActiveSequence::new(
                    &room.room,
                    t.seq.clone(),
                    &short.lua,
                    frame.time,
                )?);
                short.walk = false;
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

    let mut rng = rand::rng();
    let room = s.room(&long.room);

    let reset = if let Some(bi) = short.battle.as_mut() {
        bi.update(d, s, frame, &mut rng)
    } else {
        if let Some(seq) = short.seq.as_mut() {
            if seq.update(long, &mut short.music, &mut short.dir, s, frame)? {
                short.seq = None;
            }
        } else {
            if let Some(e_time) = short.enemy_encounter {
                if time - e_time > 1.45 {
                    short.enemy_encounter = None;
                    short.battle = Some(BattleInterface::new(time, s));
                }
            } else if update_movement(room, short, &long.flags, frame, &mut rng)? {
                s.snd("enemy_encounter").play();
                short.enemy_encounter = Some(time);
            };

            let (target_x, target_y) = short.camera_target(room);
            short.camera_x = target_x;
            short.camera_y = target_y;

            if let Some((end_time, to_room, to_transition)) = short.room_transition_to.as_ref()
                && frame.time >= *end_time
            {
                long.room = to_room.clone();
                let room = s.room(to_room);
                short.transition_to_room(room, to_transition.clone(), frame.time);
            }
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

    if let Some(bi) = short.battle.as_mut() {
        bi.draw(d, s, frame, &mut rng);
    } else {
        let room = if let Some(ro) = &short.room_override
            && long.room == ro.room
        {
            ro
        } else {
            s.room(&long.room)
        };

        let camera = Camera2D {
            target: Vector2::new(short.camera_x, short.camera_y),
            offset: Vector2::new(320.0, 240.0),
            zoom: 1.0,
            rotation: 0.0,
        };
        let mut dd = d.begin_mode2D(camera);

        room.draw_background(&mut dd, s);
        room.draw(&mut dd, s, &long.flags, None);
        /*
        dd.draw_rectangle(
            (short.pos_x - PLAYER_W / 2.0) as i32,
            (short.pos_y - PLAYER_H / 2.0) as i32,
            PLAYER_W as i32,
            PLAYER_H as i32,
            Color::BLUE,
        );
        */

        let walk_frame = if short.walk {
            ([1_usize, 0, 1, 2])[(frame.time * 8.0) as usize % 4]
        } else {
            1
        };
        let walk_dir = short.dir as usize;

        dd.draw_texture_rec(
            s.tex("enn_walk"),
            Rectangle::new(walk_frame as f32 * 24.0, walk_dir as f32 * 56.0, 24.0, 56.0),
            Vector2::new(short.pos_x - 12.0, short.pos_y - 56.0),
            Color::WHITE,
        );

        drop(dd);

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

        let fader = if let Some((to_time, _, _)) = short.room_transition_to {
            Some(((to_time - frame.time) / TRANSITION_TIME).clamp(0.0, 1.0))
        } else if frame.time < short.entered_room_at + TRANSITION_TIME {
            Some(((frame.time - short.entered_room_at) / TRANSITION_TIME).clamp(0.0, 1.0))
        } else {
            None
        };

        if let Some(fader) = fader {
            d.draw_rectangle(0, 0, 640, 480, Color::BLACK.alpha(1.0 - fader as f32));
        }
    }

    Ok(())
}
