#![allow(dead_code)]
#![warn(clippy::inconsistent_struct_constructor)]

mod dice;
mod game;
pub mod interface;
mod res;
mod test;
pub mod util;

use dice::DEFAULT_SET;
use game::content::Cnt;
use raylib::prelude::*;
use smartstring::{LazyCompact, SmartString};

use crate::game::{
    Frame, InputConfig, State, Static,
    state::{self},
};

pub type Str = SmartString<LazyCompact>;

fn main() -> eyre::Result<()> {
    let (mut rl, rt) = raylib::init()
        .size(1280, 960)
        .title("Level of Conflict")
        .build();

    let ra = RaylibAudio::init_audio_device()?;

    let res = res::load(&mut rl, &rt, &ra)?;
    let cnt = Cnt::load(&res)?;

    let s = Static {
        ra: &ra,
        res: &res,
        cnt,
    };

    let (long, short) = state::load_file(s)?;
    let input_config = InputConfig::default();

    let mut game = State { long, short };

    let mut render_texture = rl.load_render_texture(&rt, 640, 480)?;

    test::print_complete_statistics(&DEFAULT_SET);
    test::health_damage_reduction(cnt);

    let mut frame_count = 0;
    while !rl.window_should_close() {
        let frame = Frame::create(&rl, frame_count, &input_config);
        let mut d = rl.begin_drawing(&rt);

        state::update(&d, &mut game, s, frame)?;

        let mut dd = d.begin_texture_mode(&rt, &mut render_texture);
        dd.clear_background(Color::BLACK);

        state::draw(&mut dd, &mut game, s, frame)?;

        drop(dd);

        d.clear_background(Color::BLACK);

        let proposed_height = d.get_screen_height();
        let proposed_width = proposed_height * 4 / 3;
        let x_offset = d.get_screen_width() / 2 - proposed_width / 2;
        d.draw_texture_pro(
            &render_texture,
            Rectangle::new(0.0, 480.0, 640.0, -480.0),
            Rectangle::new(
                x_offset as f32,
                0.0,
                proposed_width as f32,
                proposed_height as f32,
            ),
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );

        frame_count += 1;
    }

    Ok(())
}
