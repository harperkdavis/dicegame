use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Str;

pub type Flags = HashMap<Str, i64>;

pub trait FlagsExt {
    fn is_set(&self, str: &Str) -> bool;
    fn is_not_set(&self, str: &Str) -> bool;
}

impl FlagsExt for Flags {
    fn is_set(&self, str: &Str) -> bool {
        self.get(str).is_some_and(|v| *v > 0)
    }
    fn is_not_set(&self, str: &Str) -> bool {
        !self.is_set(str)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Long {
    pub flags: HashMap<Str, i64>,
    pub room: Str,

    pub master_volume: f32,
    pub music_volume: f32,
    pub sounds_volume: f32,
}

impl Default for Long {
    fn default() -> Self {
        Self {
            flags: HashMap::new(),
            room: "home".into(),

            master_volume: 1.0,
            music_volume: 1.0,
            sounds_volume: 1.0,
        }
    }
}

impl Long {
    pub fn default_with_room(room: Str) -> Self {
        Self {
            room,
            ..Self::default()
        }
    }
}
