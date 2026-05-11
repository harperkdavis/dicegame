#![allow(dead_code)]

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

    let screen_camera = Camera2D {
        target: Vector2::new(320.0 / 2.0, 240.0 / 2.0),
        offset: Vector2::new(320.0, 240.0),
        zoom: 2.0,
        rotation: 0.0,
    };

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

        state::update(&d, &mut long, &mut short, &res, cnt);

        let mut dd = d.begin_mode2D(screen_camera);
        dd.clear_background(Color::BLACK);

        state::draw(&mut dd, &long, &mut short, &res, cnt, frame_count);

        frame_count += 1;
    }

    Ok(())
}
