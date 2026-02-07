mod dice;
mod game;
mod interface;
mod res;
mod test;
pub mod util;

use dice::DEFAULT_SET;
use interface::BattleInterface;
use raylib::prelude::*;

fn main() -> eyre::Result<()> {
    let (mut rl, rt) = raylib::init().size(1280, 960).title("Hello, World").build();
    let ra = RaylibAudio::init_audio_device()?;

    let res = res::load(&mut rl, &rt, &ra)?;

    let camera = Camera2D {
        target: Vector2::new(320.0 / 2.0, 240.0 / 2.0),
        offset: Vector2::new(320.0, 240.0),
        zoom: 2.0,
        rotation: 0.0,
    };

    test::print_complete_statistics(&DEFAULT_SET);

    let mut rng = rand::rng();

    let mut interface: BattleInterface = BattleInterface::new(0.0);
    let mut frame_count = 0;

    let mut music = res.load_mus("battle", &ra);
    music.set_volume(0.0);
    music.play_stream();
    music.looping = true;

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&rt);

        music.update_stream();

        let time = d.get_time();

        interface.update(&d, &res, &mut rng, time);

        let mut dd = d.begin_mode2D(camera);

        dd.clear_background(Color::WHITE);

        let battle_background_ocean = res.tex("battle_background_ocean");

        let mut battle_background_shader = res.sha("battle_background").borrow_mut();
        let time_loc = battle_background_shader.get_shader_location("time");
        battle_background_shader.set_shader_value(time_loc, dd.get_time() as f32);

        let mut sm = dd.begin_shader_mode(&mut battle_background_shader);

        sm.draw_texture_pro(
            battle_background_ocean,
            Rectangle::new(
                0.0,
                0.0,
                battle_background_ocean.width as f32,
                battle_background_ocean.height as f32,
            ),
            Rectangle::new(0.0, 0.0, 1280.0, 960.0),
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );
        drop(sm);

        interface.draw(&mut dd, &res, time, frame_count, &mut rng);

        frame_count += 1;
    }

    if ra.is_audio_device_ready() {
        println!("ts audio device is ready!");
    }

    Ok(())
}
