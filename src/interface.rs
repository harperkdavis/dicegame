mod attack;
mod battle;

pub use attack::AttackInterface;
pub use battle::BattleInterface;

use raylib::prelude::*;

use crate::res::Res;

#[allow(clippy::too_many_arguments)]
fn draw_die(
    dd: &mut impl RaylibDraw,
    res: &Res,
    center_x: f32,
    center_y: f32,
    rect: &Rectangle,
    outer_tint: Color,
    inner_tint: Color,
    squish: f32,
) {
    let border_tex = res.tex("dice_border");
    let dice_tex = res.tex("dice");

    dd.draw_texture_pro(
        border_tex,
        Rectangle::new(0.0, 0.0, border_tex.width as f32, border_tex.height as f32),
        Rectangle::new(
            center_x - 24.0 * squish,
            center_y - 24.0 / squish,
            48.0 * squish,
            48.0 / squish,
        ),
        Vector2::zero(),
        0.0,
        outer_tint,
    );
    dd.draw_texture_pro(
        dice_tex,
        rect,
        Rectangle::new(
            center_x - 16.0 * squish,
            center_y - 16.0 / squish,
            32.0 * squish,
            32.0 / squish,
        ),
        Vector2::zero(),
        0.0,
        inner_tint,
    );
}

pub fn health_color(health: u32, max_health: u32, time: f64) -> Color {
    let factor = health as f32 / max_health as f32;
    if health == 0 {
        Color::new(21, 21, 21, 255)
    } else if health == 1 {
        if time % 0.5 < 0.25 {
            Color::color_from_hsv(0.0, 1.0, 0.5)
        } else {
            Color::new(21, 21, 21, 255)
        }
    } else {
        Color::color_from_hsv(0.0, 1.0 - factor, 0.5 + factor * 0.5)
    }
}
