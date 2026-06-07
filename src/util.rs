use std::ops::{Add, Mul, Sub};

use raylib::{color::Color, math::Vector2, prelude::RaylibDraw, text::Font};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
#[repr(usize)]
pub enum Direction {
    #[default]
    North = 0,
    East = 1,
    South = 2,
    West = 3,
}

impl Direction {
    pub fn to_vec(&self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }
}

impl From<usize> for Direction {
    fn from(value: usize) -> Self {
        match value % 4 {
            0 => Self::North,
            1 => Self::East,
            2 => Self::South,
            3 => Self::West,
            _ => unreachable!(),
        }
    }
}

pub fn lerp<T: Copy + Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T>>(
    a: T,
    b: T,
    t: T,
) -> T {
    a + (b - a) * t
}

pub fn dlerp(a: f32, b: f32, f: f32, dt: f32) -> f32 {
    lerp(a, b, 1.0 - f.powf(dt))
}

pub fn is_within(xx: i32, yy: i32, x: i32, y: i32, w: i32, h: i32) -> bool {
    xx >= x && yy >= y && xx < x + w && yy < y + h
}

pub fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    let max_val = a.max(b);
    let min_val = a.min(b);

    min_val - (1.0 + (k * (min_val - max_val)).exp()).ln() / k
}

pub trait TextOutline {
    fn draw_text_outline(
        &mut self,
        font: &Font,
        text: &str,
        x: f32,
        y: f32,
        color: Color,
        outline: Color,
    );
}

impl<T> TextOutline for T
where
    T: RaylibDraw,
{
    fn draw_text_outline(
        &mut self,
        font: &Font,
        text: &str,
        x: f32,
        y: f32,
        color: Color,
        outline: Color,
    ) {
        for (xo, yo) in [
            (-1.0, -1.0),
            (1.0, -1.0),
            (-1.0, 1.0),
            (1.0, 1.0),
            (-1.0, 0.0),
            (0.0, -1.0),
            (1.0, 0.0),
            (0.0, 1.0),
        ] {
            self.draw_text_ex(
                font,
                text,
                Vector2::new(x.round() + xo, y.round() + yo),
                font.baseSize as f32,
                1.0,
                outline,
            );
        }
        self.draw_text_ex(
            font,
            text,
            Vector2::new(x.round(), y.round()),
            font.baseSize as f32,
            1.0,
            color,
        );
    }
}

pub trait GetSoundLength {
    fn get_sound_length_secs(&self) -> f32;
}

impl GetSoundLength for raylib::audio::Sound<'_> {
    fn get_sound_length_secs(&self) -> f32 {
        let frame_count = self.frame_count() as f32;
        let sample_rate = self.stream.sampleRate as f32;

        frame_count / sample_rate
    }
}
