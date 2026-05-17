use std::ops::{Add, Mul, Sub};

use raylib::{color::Color, math::Vector2, prelude::RaylibDraw, text::Font};

pub fn lerp<T: Copy + Add<T, Output = T> + Sub<T, Output = T> + Mul<T, Output = T>>(
    a: T,
    b: T,
    t: T,
) -> T {
    a + (b - a) * t
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
        for (xo, yo) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            self.draw_text_ex(
                font,
                text,
                Vector2::new(x.round() + xo, y.round() + yo),
                16.0,
                1.0,
                outline,
            );
        }
        self.draw_text_ex(
            font,
            text,
            Vector2::new(x.round(), y.round()),
            16.0,
            1.0,
            color,
        );
    }
}
