#![allow(dead_code)]
#![warn(clippy::inconsistent_struct_constructor)]

mod dice;
mod game;
pub mod interface;
mod res;
mod test;
pub mod util;

use dice::DEFAULT_SET;
use game::content::{Cnt, dialogue::Sequence};
use raylib::prelude::*;

use crate::game::state::{self, TICK_RATE};

struct SequenceState {
    seq: &'static Sequence,
    index: usize,
    seq_start: f64,
    line_start: Option<f64>,
}

fn main() -> eyre::Result<()> {
    let (mut rl, rt) = raylib::init()
        .size(1280, 960)
        .title("Level of Conflict")
        .build();

    let ra = RaylibAudio::init_audio_device()?;

    let res = res::load(&mut rl, &rt, &ra)?;
    let cnt = Cnt::load(&res)?;

    let (mut long, mut short) = state::load_file(&res, cnt, &ra)?;

    let mut render_texture = rl.load_render_texture(&rt, 640, 480)?;

    test::print_complete_statistics(&DEFAULT_SET);

    let mut frame_count = 0;
    let mut acc = 0.0;

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&rt);

        acc += d.get_frame_time() * TICK_RATE as f32;
        acc = acc.min(10.0);
        while acc >= 1.0 {
            state::tick(&d, &mut long, &mut short, &res, cnt);
            acc -= 1.0;
        }

        let time = d.get_time();
        state::update(&d, &mut long, &mut short, &res, cnt);

        let mut dd = d.begin_texture_mode(&rt, &mut render_texture);
        dd.clear_background(Color::BLACK);

        state::draw(&mut dd, &long, &mut short, &res, cnt, time, frame_count);

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
