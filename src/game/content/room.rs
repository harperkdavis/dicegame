use std::collections::HashMap;

use rust_embed::Embed;
use serde::Deserialize;

use crate::game::Str;

use super::{Content, dialogue::Sequence};

#[derive(Deserialize)]
pub struct Collision {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SequenceOption {
    Inline(Vec<Sequence>),
    Ref(Str),
}

#[derive(Deserialize)]
pub struct Trigger {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub auto: Option<bool>,
    pub seq: SequenceOption,
}

#[derive(Deserialize)]
pub struct SeqDef {
    pub seq: Vec<Sequence>,
}

#[derive(Deserialize)]
pub struct Layout {
    pub width: u32,
    pub height: u32,

    pub collision: Vec<Collision>,
    pub triggers: Vec<Trigger>,
}

#[derive(Deserialize)]
pub struct Room {
    pub room: String,
    pub music: String,
    pub music_pitch: Option<f64>,
    pub enemy_chance: Option<f64>,
    pub layout: Layout,
    pub sequences: HashMap<String, SeqDef>,
}

#[derive(Embed)]
#[folder = "cnt/rooms"]
pub struct RoomAsset;

impl Content for Room {
    type Context = ();
    type Asset = RoomAsset;
    fn load(_: Self::Context, _: &crate::res::Res, data: &'static [u8]) -> eyre::Result<Self> {
        // perform checks later
        toml::from_slice(data).map_err(|e| eyre::eyre!("failed to load room: {e}"))
    }
}
