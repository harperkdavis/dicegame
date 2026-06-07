use std::collections::VecDeque;

use mlua::Lua;
use raylib::prelude::RaylibDraw;

use crate::{
    Str,
    game::{
        Frame, INPUT_CANCEL, INPUT_CONFIRM, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_UP, Static,
        content::{
            Line, Room, dialogue,
            seq::{self, SeqDef},
        },
        script,
        state::{Long, music::MusicController},
    },
    interface::BattleInterface,
    util::{Direction, GetSoundLength},
};

pub struct ActiveSequence {
    started: f64,

    seq_def: SeqDef,
    lua_thread: mlua::Thread,
    resume_value: mlua::Value,

    dialogue_queue: VecDeque<Line>,
    dialogue_started: Option<f64>,

    current_line: Option<(Line, Option<f64>)>,
    current_choice: Option<(Vec<String>, usize, Option<f64>)>,

    line_is_done: bool,

    wait_until: Option<f64>,
}

impl ActiveSequence {
    pub fn new(room: &str, seq_def: SeqDef, lua: &Lua, started: f64) -> eyre::Result<Self> {
        let lua_func = script::load_sequence(room, &seq_def, lua)
            .map_err(|e| eyre::eyre!("lua error while loading sequence: {e}"))?;
        let lua_thread = lua
            .create_thread(lua_func)
            .map_err(|e| eyre::eyre!("failed to create lua thread: {e}"))?;

        Ok(Self {
            started,

            seq_def,
            lua_thread,
            resume_value: mlua::Value::Nil,

            dialogue_queue: VecDeque::new(),

            current_line: None,
            current_choice: None,
            dialogue_started: None,
            line_is_done: false,

            wait_until: None,
        })
    }

    pub fn set_resume_value(&mut self, value: mlua::Value) {
        self.resume_value = value;
    }

    pub fn get_next_event(
        thread: &mlua::Thread,
        resume_value: mlua::Value,
    ) -> eyre::Result<Option<seq::Event>> {
        match thread.resume::<Option<mlua::AnyUserData>>(resume_value) {
            Ok(Some(e)) => match e.borrow::<seq::Event>() {
                Ok(e) => Ok(Some(e.to_owned())),
                Err(e) => Err(eyre::eyre!("sequence coroutine yielded non-event: {e}")),
            },
            Ok(None) => {
                if thread.status() != mlua::ThreadStatus::Resumable {
                    Ok(None)
                } else {
                    Err(eyre::eyre!(
                        "sequence coroutine did not yield event but thread did not complete"
                    ))
                }
            }
            Err(e) => Err(eyre::eyre!("lua error: {e}")),
        }
    }

    // returns Ok(true) if complete
    pub fn update(
        &mut self,
        long: &mut Long,
        mc: &mut MusicController,
        dir: &mut Direction,
        s: Static,
        frame: Frame,
    ) -> eyre::Result<bool> {
        if let Some(until) = self.wait_until {
            if frame.time >= until {
                // sequence will resume next frame.
                self.wait_until = None;
            }
            return Ok(false);
        } else if let Some((options, current_choice, choice_updated)) = self.current_choice.as_mut()
        {
            let prev_choice = *current_choice;
            if frame.actions_down[INPUT_LEFT] {
                *current_choice += options.len() - 1;
            }
            if frame.actions_down[INPUT_RIGHT] {
                *current_choice += 1;
            }
            if frame.actions_down[INPUT_UP] {
                *current_choice += options.len() * 2 - 2;
            }
            if frame.actions_down[INPUT_DOWN] {
                *current_choice += 2;
            }
            *current_choice %= options.len();
            if *current_choice != prev_choice {
                s.snd("menu").play();
                *choice_updated = Some(frame.time);
            }
            if frame.actions_down[INPUT_CONFIRM] {
                s.snd("select").play();
                self.resume_value = mlua::Value::Integer(*current_choice as i64);
                self.current_choice = None;
            }
        } else if let Some((_, line_started)) = self.current_line.as_mut() {
            // currently handling a line
            if line_started.is_some() {
                if frame.actions_down[INPUT_CANCEL] {
                    *line_started = None;
                }
            } else {
                if frame.actions_down[INPUT_CONFIRM] {
                    self.current_line = None;
                }
            }
        } else if !self.dialogue_queue.is_empty() && self.current_line.is_none() {
            // not currently handling a line. could use another.
            let next = self.dialogue_queue.pop_front().unwrap();
            self.current_line = Some((next, Some(frame.time)));
        } else {
            loop {
                if let Some(e) = Self::get_next_event(&self.lua_thread, self.resume_value.clone())?
                {
                    match e {
                        seq::Event::GetFlag(id) => {
                            let value = *long.flags.entry(id.into()).or_insert(0);
                            self.resume_value = mlua::Value::Integer(value);
                        }
                        seq::Event::SetFlag(id, value) => {
                            long.flags.insert(id.into(), value);
                            self.resume_value = mlua::Value::Integer(value);
                        }
                        seq::Event::Wait(secs) => {
                            self.wait_until = Some(frame.time + secs as f64);
                            self.resume_value = mlua::Value::Nil;
                            break;
                        }
                        seq::Event::Write(mut lines) => {
                            self.dialogue_queue.extend(lines.drain(..));
                            self.resume_value = mlua::Value::Nil;
                            self.dialogue_started = Some(frame.time);
                            break;
                        }
                        seq::Event::Choice(options) => {
                            self.dialogue_started = Some(frame.time);
                            self.current_choice = Some((options, 0, None));
                            break;
                        }
                        seq::Event::SetMusic(music) => {
                            mc.set_current_music(music);
                            self.resume_value = mlua::Value::Nil;
                        }
                        seq::Event::PlaySound(sound) => {
                            s.snd(&sound).play();
                            self.resume_value = mlua::Value::Nil;
                        }
                        seq::Event::PlaySoundAndWait(sound, wait) => {
                            let sound = s.snd(&sound);
                            sound.play();
                            if let Some(wait) = wait {
                                self.wait_until = Some(frame.time + wait as f64);
                            } else {
                                self.wait_until =
                                    Some(frame.time + sound.get_sound_length_secs() as f64)
                            }
                            self.resume_value = mlua::Value::Nil;
                            break;
                        }
                        seq::Event::SetDirection(d) => {
                            let new_dir = Direction::from(d as usize);
                            *dir = new_dir;
                            self.resume_value = mlua::Value::Nil;
                        }
                    }
                } else {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn draw(&mut self, d: &mut impl RaylibDraw, s: Static, frame: Frame) {
        if let Some((choices, current_choice, select_time)) = self.current_choice.as_ref() {
            dialogue::draw_choice_box(
                d,
                s.fnt("default2"),
                frame.time - self.dialogue_started.unwrap(),
                select_time.map(|e| frame.time - e),
                choices,
                *current_choice,
            );
        } else if let Some((line, line_started)) = self.current_line.as_mut() {
            let finished = line.draw(
                d,
                s.res,
                s.fnt("default2"),
                frame.time - self.dialogue_started.unwrap(),
                line_started.map(|e| frame.time - e),
                frame.delta,
            );
            if finished {
                *line_started = None;
            }
        }
    }
}

pub struct Short<'a> {
    pub(super) lua: Lua,

    pub(super) pos_x: f32,
    pub(super) pos_y: f32,

    pub(super) camera_x: f32,
    pub(super) camera_y: f32,

    pub(super) dir: Direction,
    pub(super) walk: bool,

    pub(super) music: MusicController<'a>,

    pub(super) enemy_encounter: Option<f64>,
    pub(super) battle: Option<BattleInterface>,
    pub(super) battle_result: Option<bool>,

    pub(super) seq: Option<ActiveSequence>,

    pub(super) room_transition_to: Option<(f64, Str, Option<Str>)>,
    pub(super) entered_room_at: f64,
    pub(super) room_override: Option<Room>,
}

impl<'a> Short<'a> {
    fn constrain_camera_target(pos_x: f32, pos_y: f32, room: &Room) -> (f32, f32) {
        let (x_bounds, y_bounds) = room.camera_bounds();
        let x_pos = if let Some((min_x, max_x)) = x_bounds {
            pos_x.clamp(min_x as f32, max_x as f32)
        } else {
            320.0
        };
        let y_pos = if let Some((min_y, max_y)) = y_bounds {
            pos_y.clamp(min_y as f32, max_y as f32)
        } else {
            240.0
        };
        (x_pos, y_pos)
    }
    pub fn camera_target(&self, room: &Room) -> (f32, f32) {
        Self::constrain_camera_target(self.pos_x, self.pos_y, room)
    }

    pub fn transition_to_room(&mut self, room: &Room, to_transition: Option<Str>, time: f64) {
        let (pos_x, pos_y, dir) = room.get_start_pos(to_transition.as_ref());
        (self.pos_x, self.pos_y, self.dir) = (pos_x as f32, pos_y as f32, dir);
        self.music.set_current_music(room.music.clone());
        self.entered_room_at = time;
        self.room_transition_to = None;
        (self.camera_x, self.camera_y) =
            Self::constrain_camera_target(self.pos_x, self.pos_y, room);
    }

    pub fn from_long(
        long: &Long,
        s: Static<'a>,
        room_override: Option<Room>,
    ) -> eyre::Result<Self> {
        let lua =
            script::create_context().map_err(|e| eyre::eyre!("lua error creating context: {e}"))?;
        let mut music = MusicController::load(s, 0.2)?;

        let room = room_override.as_ref().unwrap_or(s.room(&long.room));
        let (pos_x, pos_y, dir) = room.get_start_pos(None);
        let (camera_x, camera_y) = Self::constrain_camera_target(pos_x as f32, pos_y as f32, room);

        music.set_current_music(room.music.clone());

        Ok(Self {
            lua,

            music,

            pos_x: pos_x as f32,
            pos_y: pos_y as f32,

            camera_x,
            camera_y,

            dir,
            walk: false,

            enemy_encounter: None,
            battle: None,
            battle_result: None,

            seq: None,

            room_transition_to: None,
            entered_room_at: 0.0,
            room_override,
        })
    }
}
