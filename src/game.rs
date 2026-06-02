pub mod battle;
pub mod content;
pub mod script;
pub mod state;

use std::{borrow::Borrow, fmt::Debug, hash::Hash};

use content::Cnt;
pub use content::Content;
use raylib::{
    RaylibHandle,
    audio::{RaylibAudio, Sound},
    ffi::KeyboardKey,
    text::Font,
    texture::Texture2D,
};
use serde::{Deserialize, Serialize};
pub use state::State;

use crate::{
    Str,
    game::{
        battle::{EnemyDef, ItemDef, PartyDef},
        content::Room,
    },
    res::{Res, ShaderPtr},
};

#[derive(Clone, Copy)]
pub struct Static<'a> {
    pub ra: &'a RaylibAudio,
    pub res: &'a Res<'a>,
    pub cnt: Cnt,
}

impl<'a> Static<'a> {
    pub fn tex<K>(&self, k: &K) -> &Texture2D
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        self.res.tex(k)
    }

    pub fn snd<K>(&self, k: &K) -> &Sound<'a>
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        self.res.snd(k)
    }

    pub fn fnt<K>(&self, k: &K) -> &Font
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        self.res.fnt(k)
    }

    pub fn sha<K>(&self, k: &K) -> &ShaderPtr
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        self.res.sha(k)
    }

    pub fn room<K>(&self, k: &K) -> &'static Room
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        &self.cnt.rooms[k]
    }

    pub fn item<K>(&self, k: &K) -> &'static ItemDef
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        &self.cnt.items[k]
    }

    pub fn party<K>(&self, k: &K) -> &'static PartyDef
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        &self.cnt.party[k]
    }

    pub fn enemy<K>(&self, k: &K) -> &'static EnemyDef
    where
        K: Eq + Hash + Debug + ?Sized,
        Str: Borrow<K>,
    {
        &self.cnt.enemies[k]
    }
}

pub const INPUT_COUNT: usize = 7;

pub const INPUT_UP: usize = 0;
pub const INPUT_LEFT: usize = 1;
pub const INPUT_DOWN: usize = 2;
pub const INPUT_RIGHT: usize = 3;

pub const INPUT_CONFIRM: usize = 4;
pub const INPUT_CANCEL: usize = 5;
pub const INPUT_MENU: usize = 6;

#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub time: f64,
    pub frame_count: usize,
    pub delta: f32,

    pub input_x: f32,
    pub input_y: f32,

    pub actions: [bool; INPUT_COUNT],
    pub actions_down: [bool; INPUT_COUNT],
    pub actions_up: [bool; INPUT_COUNT],
}

#[derive(Serialize, Deserialize)]
pub struct InputConfig {
    binds: [u32; INPUT_COUNT],
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            binds: [
                key_to_u32(KeyboardKey::KEY_UP),
                key_to_u32(KeyboardKey::KEY_LEFT),
                key_to_u32(KeyboardKey::KEY_DOWN),
                key_to_u32(KeyboardKey::KEY_RIGHT),
                key_to_u32(KeyboardKey::KEY_Z),
                key_to_u32(KeyboardKey::KEY_X),
                key_to_u32(KeyboardKey::KEY_C),
            ],
        }
    }
}

pub fn u32_to_key(key: u32) -> KeyboardKey {
    unsafe { std::mem::transmute(key) }
}

pub const fn key_to_u32(key: KeyboardKey) -> u32 {
    key as u32
}

impl Frame {
    pub fn create(d: &RaylibHandle, frame_count: usize, input_config: &InputConfig) -> Self {
        let time = d.get_time();
        let delta = d.get_frame_time();

        let mut actions = [false; INPUT_COUNT];
        let mut actions_down = [false; INPUT_COUNT];
        let mut actions_up = [false; INPUT_COUNT];

        for i in 0..INPUT_COUNT {
            let bind = u32_to_key(input_config.binds[i]);
            actions[i] = d.is_key_down(bind);
            actions_down[i] = d.is_key_pressed(bind);
            actions_up[i] = d.is_key_released(bind);
        }

        let input_x = if actions[INPUT_LEFT] { -1.0 } else { 0.0 }
            + if actions[INPUT_RIGHT] { 1.0 } else { 0.0 };
        let input_y = if actions[INPUT_UP] { -1.0 } else { 0.0 }
            + if actions[INPUT_DOWN] { 1.0 } else { 0.0 };

        Self {
            time,
            frame_count,
            delta,

            input_x,
            input_y,

            actions,
            actions_down,
            actions_up,
        }
    }
}
