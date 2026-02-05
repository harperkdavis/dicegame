mod attack;

pub use attack::AttackInterface;

use crate::assets::Assets;
use raylib::prelude::*;

#[allow(clippy::too_many_arguments)]
fn draw_die(
    dd: &mut impl RaylibDraw,
    assets: &Assets,
    center_x: f32,
    center_y: f32,
    rect: &Rectangle,
    outer_tint: Color,
    inner_tint: Color,
    squish: f32,
) {
    let border_tex = assets.get_texture("dice_border");
    let dice_tex = assets.get_texture("dice_textured");

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
