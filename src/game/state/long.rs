use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::Str;

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
