use std::collections::HashMap;

use raylib::{color::Color, prelude::RaylibDraw};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};

use crate::{
    Str,
    game::{Static, content::seq::SeqDef},
};

use super::Content;

#[derive(Deserialize, Serialize)]
pub struct Collision {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Deserialize, Serialize)]
pub struct Trigger {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub auto: Option<bool>,
    pub seq: SeqDef,
}

#[derive(Deserialize, Serialize)]
pub struct Layout {
    pub width: u32,
    pub height: u32,

    pub collision: Vec<Collision>,
    pub triggers: Vec<Trigger>,
}

#[derive(Deserialize, Serialize)]
pub struct Room {
    pub room: String,
    pub music: Option<String>,
    pub music_pitch: Option<f64>,
    pub enemy_chance: Option<f64>,
    pub background: Option<Str>,
    pub layout: Layout,
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

impl Room {
    pub fn draw(&self, d: &mut impl RaylibDraw, s: Static) {
        for t in &self.layout.triggers {
            d.draw_rectangle(t.x, t.y, t.w, t.h, Color::DARKBLUE);
        }
    }
}
