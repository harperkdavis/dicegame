use std::collections::VecDeque;

use mlua::Lua;
use raylib::prelude::RaylibDraw;

use crate::{
    game::{
        Frame, INPUT_CANCEL, INPUT_CONFIRM, Static,
        content::{
            Line,
            seq::{self, SeqDef},
        },
        script,
        state::{Long, music::MusicController},
    },
    interface::BattleInterface,
};

pub struct ActiveSequence {
    started: f64,

    seq_def: SeqDef,
    lua_thread: mlua::Thread,
    resume_value: mlua::Value,

    dialogue_queue: VecDeque<Line>,
    dialogue_started: Option<f64>,
    current_line: Option<(Line, Option<f64>)>,
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
    pub fn update(&mut self, long: &mut Long, frame: Frame) -> eyre::Result<bool> {
        if let Some(until) = self.wait_until {
            if frame.time >= until {
                // sequence will resume next frame.
                self.wait_until = None;
            }
            return Ok(false);
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
                        _ => todo!(),
                    }
                } else {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn draw(&mut self, d: &mut impl RaylibDraw, s: Static, frame: Frame) {
        if let Some((line, line_started)) = self.current_line.as_mut() {
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

    pub(super) music: MusicController<'a>,

    pub(super) enemy_encounter: Option<f64>,
    pub(super) battle: Option<BattleInterface>,
    pub(super) battle_result: Option<bool>,

    pub(super) seq: Option<ActiveSequence>,
}

impl<'a> Short<'a> {
    pub fn from_long(_long: &Long, s: Static<'a>) -> eyre::Result<Self> {
        let lua =
            script::create_context().map_err(|e| eyre::eyre!("lua error creating context: {e}"))?;
        let music = MusicController::load(s, 0.2)?;

        Ok(Self {
            lua,

            pos_x: 0.0,
            pos_y: 0.0,

            music,

            enemy_encounter: None,
            battle: None,
            battle_result: None,

            seq: None,
        })
    }
}
