use std::num::NonZeroU32;

use raylib::{
    color::Color,
    math::{Rectangle, Vector2},
    prelude::RaylibDraw,
};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::{
    Str,
    game::{Static, content::seq::SeqDef},
    util,
};

use super::Content;

#[derive(Clone, Copy, IntoStaticStr, PartialEq, Eq)]
#[repr(usize)]
pub enum Layer {
    Object,
    Trigger,
    Collision,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LayerItem {
    pub layer: Layer,
    pub item: usize,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: NonZeroU32,
    pub h: NonZeroU32,
}

impl Rect {
    pub fn is_within(&self, xx: i32, yy: i32) -> bool {
        util::is_within(
            xx,
            yy,
            self.x,
            self.y,
            u32::from(self.w).cast_signed(),
            u32::from(self.h).cast_signed(),
        )
    }

    pub fn outline(&self) -> Rect {
        Rect {
            x: self.x - 1,
            y: self.y - 1,
            w: self.w.saturating_add(2),
            h: self.h.saturating_add(2),
        }
    }

    pub fn bottom_y(&self) -> i32 {
        self.y.saturating_add_unsigned(u32::from(self.h))
    }

    pub fn auto_collision(&self) -> Rect {
        Rect {
            x: self.x - 2,
            y: self.bottom_y() - 16,
            w: self.w.saturating_add(4),
            h: NonZeroU32::new(18).unwrap(),
        }
    }
}

impl From<Rect> for Rectangle {
    fn from(value: Rect) -> Self {
        Rectangle::new(
            value.x as f32,
            value.y as f32,
            u32::from(value.w) as f32,
            u32::from(value.h) as f32,
        )
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Collision {
    pub r: Rect,
    pub disabled_by_flag: Option<Str>,
}

impl Collision {
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            r: rect,
            disabled_by_flag: None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Trigger {
    pub r: Rect,
    pub inner: Option<bool>,
    pub auto: Option<bool>,
    pub seq: SeqDef,
}

impl Trigger {
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            r: rect,
            inner: None,
            auto: None,
            seq: SeqDef::Simple(Box::new(["(PLACEHOLDER)".to_string()])),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Object {
    pub r: Rect,
    pub sprite: Str,
    pub name: Option<Str>,
}

impl Object {
    pub fn new_from_texture(place_pos: (i32, i32), sprite: Str, s: Static) -> Self {
        let tex = s.tex(&sprite);
        let (x, y) = place_pos;
        Self {
            r: Rect {
                x,
                y,
                w: NonZeroU32::new(tex.width.max(1) as u32).unwrap(),
                h: NonZeroU32::new(tex.height.max(1) as u32).unwrap(),
            },
            sprite,
            name: None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct Layout {
    pub width: u32,
    pub height: u32,

    pub objects: Vec<Object>,
    pub collision: Vec<Collision>,
    pub triggers: Vec<Trigger>,
}

impl Layout {
    pub fn overlapping_objects(&self, xx: i32, yy: i32) -> impl Iterator<Item = usize> {
        self.objects
            .iter()
            .enumerate()
            .rev()
            .filter_map(move |(i, o)| o.r.is_within(xx, yy).then_some(i))
    }

    pub fn overlapping_collision(&self, xx: i32, yy: i32) -> impl Iterator<Item = usize> {
        self.collision
            .iter()
            .enumerate()
            .filter_map(move |(i, c)| c.r.is_within(xx, yy).then_some(i))
    }

    pub fn overlapping_triggers(&self, xx: i32, yy: i32) -> impl Iterator<Item = usize> {
        self.triggers
            .iter()
            .enumerate()
            .filter_map(move |(i, t)| t.r.is_within(xx, yy).then_some(i))
    }
}

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct Room {
    pub room: String,
    pub music: Option<Str>,
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
    pub fn draw_background(&self, d: &mut impl RaylibDraw, s: Static) {
        if let Some(bg) = self.background.as_ref() {
            let bg = s.tex(bg);
            d.draw_texture_rec(
                bg,
                Rectangle::new(0.0, 0.0, bg.width as f32 * 128.0, bg.height as f32 * 128.0),
                Vector2::new(-bg.width as f32 * 63.0, -bg.height as f32 * 63.0),
                Color::WHITE,
            );
        }
    }

    pub fn re_sort_y(&mut self) {
        self.layout
            .objects
            .sort_by(|a, b| i32::cmp(&a.r.bottom_y(), &b.r.bottom_y()));
    }

    pub fn add_object(&mut self, object: Object) -> usize {
        if let Some(i) = self.layout.objects.iter().position(|o| o.r == object.r) {
            i
        } else {
            self.layout.objects.push(object);
            self.layout.objects.len() - 1
        }
    }

    pub fn add_collision(&mut self, rect: Rect) -> usize {
        let collision = Collision::from_rect(rect);
        if let Some(i) = self
            .layout
            .collision
            .iter()
            .position(|c| c.r == collision.r)
        {
            i
        } else {
            self.layout.collision.push(collision);
            self.layout.collision.len() - 1
        }
    }

    pub fn add_trigger(&mut self, rect: Rect) -> usize {
        let trigger = Trigger::from_rect(rect);
        if let Some(i) = self.layout.triggers.iter().position(|t| t.r == trigger.r) {
            i
        } else {
            self.layout.triggers.push(trigger);
            self.layout.triggers.len() - 1
        }
    }

    pub fn remove(&mut self, li: LayerItem) {
        match li.layer {
            Layer::Object => {
                self.layout.objects.remove(li.item);
            }
            Layer::Collision => {
                self.layout.collision.remove(li.item);
            }
            Layer::Trigger => {
                self.layout.triggers.remove(li.item);
            }
        }
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        s: Static,
        debug: Option<(Option<LayerItem>, Option<LayerItem>)>,
    ) {
        let mut hover_rect = None;
        let mut selected_rect = None;

        let mut object_refs = self.layout.objects.iter().enumerate().collect::<Box<[_]>>();
        object_refs.sort_by(|(_, a), (_, b)| i32::cmp(&a.r.bottom_y(), &b.r.bottom_y()));

        for (i, o) in object_refs {
            let li = LayerItem {
                layer: Layer::Object,
                item: i,
            };
            if let Some((hover, selected)) = debug {
                if hover == Some(li) {
                    hover_rect = Some(o.r);
                }
                if selected == Some(li) {
                    selected_rect = Some(o.r);
                }
            }

            let tex = s.tex(&o.sprite);
            d.draw_texture_pro(
                tex,
                Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                Rectangle::from(o.r),
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
        }

        if let Some((hover, selected)) = debug {
            for (i, t) in self.layout.triggers.iter().enumerate() {
                let li = LayerItem {
                    layer: Layer::Trigger,
                    item: i,
                };
                if hover == Some(li) {
                    hover_rect = Some(t.r);
                }
                if selected == Some(li) {
                    selected_rect = Some(t.r);
                }
                d.draw_rectangle(
                    t.r.x,
                    t.r.y,
                    u32::from(t.r.w) as i32,
                    u32::from(t.r.h) as i32,
                    Color::BLUE.alpha(0.5),
                );
            }
            for (i, c) in self.layout.collision.iter().enumerate() {
                let li = LayerItem {
                    layer: Layer::Collision,
                    item: i,
                };
                if hover == Some(li) {
                    hover_rect = Some(c.r);
                }
                if selected == Some(li) {
                    selected_rect = Some(c.r);
                }

                d.draw_rectangle(
                    c.r.x,
                    c.r.y,
                    u32::from(c.r.w) as i32,
                    u32::from(c.r.h) as i32,
                    Color::LIME.alpha(0.5),
                );
            }
        }

        if let Some(r) = selected_rect.map(|r| r.outline()) {
            d.draw_rectangle_lines(
                r.x,
                r.y,
                u32::from(r.w) as i32,
                u32::from(r.h) as i32,
                Color::YELLOW,
            );
        }
        if let Some(r) = hover_rect.map(|r| r.outline()) {
            d.draw_rectangle_lines(
                r.x,
                r.y,
                u32::from(r.w) as i32,
                u32::from(r.h) as i32,
                Color::WHITE.alpha(0.5),
            );
        }
    }
}
