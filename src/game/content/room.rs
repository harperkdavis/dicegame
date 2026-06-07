use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    num::NonZeroU32,
};

use raylib::{
    color::Color,
    math::{Rectangle, Vector2},
    prelude::RaylibDraw,
};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use toml_edit::{DocumentMut, visit_mut::VisitMut};

use crate::{
    Str,
    game::{
        Static,
        content::seq::SeqDef,
        state::long::{Flags, FlagsExt},
    },
    util::{self, Direction},
};

use super::Content;

#[derive(Clone, Copy, IntoStaticStr, PartialEq, Eq)]
#[repr(usize)]
pub enum Layer {
    Object,
    Trigger,
    Collision,
    Transition,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LayerItem {
    pub layer: Layer,
    pub item: usize,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: NonZeroU32,
    pub h: NonZeroU32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Props {
    pub enable_if_flag: Option<Str>,
    pub disable_if_flag: Option<Str>,
}

impl Props {
    pub fn is_enabled(&self, flags: &Flags) -> bool {
        self.disable_if_flag
            .as_ref()
            .is_none_or(|f| flags.is_not_set(f))
            && self.enable_if_flag.as_ref().is_none_or(|f| flags.is_set(f))
    }
}

pub trait PropsExt {
    fn is_enabled(&self, flags: &Flags) -> bool;
}

impl PropsExt for Option<Props> {
    fn is_enabled(&self, flags: &Flags) -> bool {
        self.as_ref().is_none_or(|s| s.is_enabled(flags))
    }
}

impl Rect {
    pub fn unique(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

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

    pub fn is_within_f(&self, xx: f32, yy: f32) -> bool {
        let x = self.x as f32;
        let y = self.y as f32;
        let w = u32::from(self.w) as f32;
        let h = u32::from(self.h) as f32;
        xx >= x && yy >= y && xx < x + w && yy < y + h
    }

    pub fn outline(&self) -> Rect {
        Rect {
            x: self.x - 1,
            y: self.y - 1,
            w: self.w.saturating_add(2),
            h: self.h.saturating_add(2),
        }
    }

    pub fn from_points(a: (i32, i32), b: (i32, i32)) -> Self {
        Self {
            x: a.0.min(b.0),
            y: a.1.min(b.1),
            w: NonZeroU32::new(a.0.abs_diff(b.0).max(1)).unwrap(),
            h: NonZeroU32::new(a.1.abs_diff(b.1).max(1)).unwrap(),
        }
    }

    pub fn midpoint(&self) -> (i32, i32) {
        (
            self.x.midpoint(self.x + u32::from(self.w).cast_signed()),
            self.y.midpoint(self.y + u32::from(self.h).cast_signed()),
        )
    }

    pub fn bottom_y(&self) -> i32 {
        self.y.saturating_add_unsigned(u32::from(self.h))
    }

    pub fn right_x(&self) -> i32 {
        self.x.saturating_add_unsigned(u32::from(self.w))
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
    pub props: Option<Props>,
}

impl Collision {
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            r: rect,
            props: None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Trigger {
    pub r: Rect,
    pub props: Option<Props>,
    pub inner: Option<bool>,
    pub auto: Option<bool>,
    pub seq: SeqDef,
}

impl Trigger {
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            r: rect,
            props: None,
            inner: None,
            auto: None,
            seq: SeqDef::Simple(Box::new(["(PLACEHOLDER)".to_string()])),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Object {
    pub r: Rect,
    pub props: Option<Props>,
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
            props: None,
            sprite,
            name: None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Transition {
    pub r: Rect,
    pub props: Option<Props>,
    pub name: Str,
    pub enter_dir: Option<Direction>,
    pub to_room: Option<Str>,
    pub to_transition: Option<Str>,
    pub default: bool,
    pub is_save_point: bool,
}

impl Transition {
    pub fn from_rect(rect: Rect, default: bool) -> Self {
        Self {
            r: rect,
            props: None,
            name: format!("{:x}", rect.unique()).into(),
            enter_dir: None,
            to_room: Some("(PLACEHOLDER)".into()),
            to_transition: Some("(PLACEHOLDER)".into()),
            default,
            is_save_point: false,
        }
    }

    pub fn save_point_at(pos: (i32, i32), room: Str) -> Self {
        let rect = Rect {
            x: pos.0 - 10,
            y: pos.1 - 10,
            w: NonZeroU32::new(20).unwrap(),
            h: NonZeroU32::new(20).unwrap(),
        };
        Self {
            r: rect,
            props: None,
            name: format!("{room}/save").into(),
            enter_dir: Some(Direction::South),
            to_room: None,
            to_transition: None,
            default: true,
            is_save_point: true,
        }
    }

    pub fn priority(&self, priority: Option<&Str>) -> usize {
        (if let Some(p) = priority
            && &self.name == p
        {
            4
        } else {
            0
        }) + (if self.is_save_point { 2 } else { 0 })
            + (if self.default { 1 } else { 0 })
    }
}

#[derive(Clone, Deserialize, Serialize, Default)]
pub struct Layout {
    pub left_x: i32,
    pub right_x: i32,
    pub top_y: i32,
    pub bottom_y: i32,

    pub objects: Vec<Object>,
    pub collision: Vec<Collision>,
    pub triggers: Vec<Trigger>,
    pub transitions: Vec<Transition>,
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

    pub fn overlapping_transitions(&self, xx: i32, yy: i32) -> impl Iterator<Item = usize> {
        self.transitions
            .iter()
            .enumerate()
            .filter_map(move |(i, n)| n.r.is_within(xx, yy).then_some(i))
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
        toml_edit::de::from_slice(data).map_err(|e| eyre::eyre!("failed to load room: {e}"))
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

    pub fn refresh(&mut self, s: Static) {
        self.layout
            .objects
            .sort_by(|a, b| i32::cmp(&a.r.bottom_y(), &b.r.bottom_y()));

        let (mut max_x, mut max_y) = if let Some(bg) = &self.background {
            let tex = s.tex(bg);
            (tex.width, tex.height)
        } else {
            (0, 0)
        };
        let mut min_x = 0;
        let mut min_y = 0;

        for r in self
            .layout
            .collision
            .iter()
            .map(|c| c.r)
            .chain(self.layout.triggers.iter().map(|t| t.r))
            .chain(self.layout.objects.iter().map(|o| o.r))
            .chain(self.layout.transitions.iter().map(|n| n.r))
        {
            min_x = min_x.min(r.x);
            min_y = min_y.min(r.y);
            max_x = max_x.max(r.right_x());
            max_y = max_y.max(r.bottom_y());
        }
    }

    pub fn camera_bounds(&self) -> (Option<(i32, i32)>, Option<(i32, i32)>) {
        let x_bounds = if self.layout.left_x + 320 >= self.layout.right_x - 320 {
            None
        } else {
            Some((self.layout.left_x + 320, self.layout.right_x - 320))
        };
        let y_bounds = if self.layout.top_y + 240 >= self.layout.bottom_y - 240 {
            None
        } else {
            Some((self.layout.top_y + 240, self.layout.bottom_y - 240))
        };
        (x_bounds, y_bounds)
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

    pub fn add_transition(&mut self, rect: Rect) -> usize {
        let trans = Transition::from_rect(rect, self.layout.transitions.is_empty());
        if let Some(i) = self.layout.transitions.iter().position(|t| t.r == trans.r) {
            i
        } else {
            self.layout.transitions.push(trans);
            self.layout.transitions.len() - 1
        }
    }

    pub fn add_save_point(&mut self, pos: (i32, i32), room: Str) -> usize {
        let trans = Transition::save_point_at(pos, room);
        if let Some(i) = self.layout.transitions.iter().position(|n| n.r == trans.r) {
            i
        } else {
            self.layout.transitions.push(trans);
            self.layout.transitions.len() - 1
        }
    }

    pub fn add_rect(&mut self, rect: Rect, layer: Layer) -> usize {
        match layer {
            // no-op
            Layer::Object => todo!("no behavior for adding object rect"),
            Layer::Collision => self.add_collision(rect),
            Layer::Trigger => self.add_trigger(rect),
            Layer::Transition => self.add_transition(rect),
        }
    }

    pub fn get_rect(&self, li: LayerItem) -> Rect {
        match li.layer {
            Layer::Object => self.layout.objects[li.item].r,
            Layer::Collision => self.layout.collision[li.item].r,
            Layer::Trigger => self.layout.triggers[li.item].r,
            Layer::Transition => self.layout.transitions[li.item].r,
        }
    }

    pub fn get_rect_mut(&mut self, li: LayerItem) -> &mut Rect {
        match li.layer {
            Layer::Object => &mut self.layout.objects[li.item].r,
            Layer::Collision => &mut self.layout.collision[li.item].r,
            Layer::Trigger => &mut self.layout.triggers[li.item].r,
            Layer::Transition => &mut self.layout.transitions[li.item].r,
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
            Layer::Transition => {
                self.layout.transitions.remove(li.item);
            }
        }
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        s: Static,
        flags: &Flags,
        debug: Option<(Option<LayerItem>, Option<LayerItem>)>,
    ) {
        let mut hover_rect = None;
        let mut selected_rect = None;

        let mut object_refs = self.layout.objects.iter().enumerate().collect::<Box<[_]>>();
        object_refs.sort_by(|(_, a), (_, b)| i32::cmp(&a.r.bottom_y(), &b.r.bottom_y()));

        for (i, o) in object_refs {
            if debug.is_none() && !o.props.is_enabled(flags) {
                continue;
            }
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
            let debug = self
                .layout
                .triggers
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    (
                        LayerItem {
                            layer: Layer::Trigger,
                            item: i,
                        },
                        t.r,
                        Color::BLUE.alpha(0.5),
                    )
                })
                .chain(self.layout.collision.iter().enumerate().map(|(i, c)| {
                    (
                        LayerItem {
                            layer: Layer::Collision,
                            item: i,
                        },
                        c.r,
                        Color::LIME.alpha(0.5),
                    )
                }))
                .chain(self.layout.transitions.iter().enumerate().map(|(i, n)| {
                    (
                        LayerItem {
                            layer: Layer::Transition,
                            item: i,
                        },
                        n.r,
                        if n.default {
                            Color::ORANGE.alpha(0.5)
                        } else {
                            Color::RED.alpha(0.5)
                        },
                    )
                }));
            for (li, r, color) in debug {
                if hover == Some(li) {
                    hover_rect = Some(r);
                }
                if selected == Some(li) {
                    selected_rect = Some(r);
                }
                d.draw_rectangle(
                    r.x,
                    r.y,
                    u32::from(r.w) as i32,
                    u32::from(r.h) as i32,
                    color,
                );
            }

            for n in self
                .layout
                .transitions
                .iter()
                .filter(|n| n.enter_dir.is_some())
            {
                let ed = n.enter_dir.unwrap();
                let (x, y) = n.r.midpoint();
                let (xx, yy) = ed.to_vec();

                d.draw_line(x, y, x + xx * 20, y + yy * 20, Color::RED);
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

    pub fn get_start_pos(&self, specified: Option<&Str>) -> (i32, i32, Direction) {
        if self.layout.transitions.is_empty() {
            (0, 0, Direction::default())
        } else if self.layout.transitions.len() == 1 {
            let (x, y) = self.layout.transitions[0].r.midpoint();
            (
                x,
                y,
                self.layout.transitions[0].enter_dir.unwrap_or_default(),
            )
        } else {
            let mut possible = self.layout.transitions.iter().collect::<Box<[_]>>();
            possible.sort_by(|a, b| a.priority(specified).cmp(&b.priority(specified)).reverse());
            let (x, y) = possible[0].r.midpoint();
            (x, y, possible[0].enter_dir.unwrap_or_default())
        }
    }

    pub fn ser(&self) -> eyre::Result<String> {
        let pretty_str = toml_edit::ser::to_string_pretty(self)?;

        let mut doc = pretty_str
            .parse::<DocumentMut>()
            .map_err(|e| eyre::eyre!("serialization error: {e}"))?;

        let mut visitor = InlineVisitor;
        visitor.visit_document_mut(&mut doc);

        Ok(doc.to_string())
    }
}

struct InlineVisitor;

impl VisitMut for InlineVisitor {
    fn visit_table_like_kv_mut(&mut self, _: toml_edit::KeyMut<'_>, node: &mut toml_edit::Item) {
        use toml_edit::{Item, visit_mut};
        if let Item::ArrayOfTables(array_of_tables) = node {
            for table in array_of_tables.iter_mut() {
                if let Some(r_table) = table.get_mut("r").and_then(|i| i.as_table_mut()) {
                    let inline = r_table.clone().into_inline_table();
                    table.insert("r", Item::Value(toml_edit::Value::InlineTable(inline)));
                }
                if let Some(props_table) = table.get_mut("props").and_then(|p| p.as_table_mut()) {
                    let inline = props_table.clone().into_inline_table();
                    table.insert("props", Item::Value(toml_edit::Value::InlineTable(inline)));
                }
            }
        }

        visit_mut::visit_item_mut(self, node);
    }
}
