mod assets;
mod dice;
mod interface;
mod test;
mod util;

use assets::Assets;
use dice::DEFAULT_SET;
use interface::AttackInterface;
use raylib::prelude::*;

fn main() -> eyre::Result<()> {
    let (mut rl, rt) = raylib::init().size(1280, 960).title("Hello, World").build();
    let ra = RaylibAudio::init_audio_device()?;

    let mut music = ra
        .new_music("assets/music/battle.wav")
        .expect("could not load battle music");

    let assets = Assets::load(&mut rl, &rt, &ra)?;

    let mut battle_background_shader =
        rl.load_shader(&rt, None, Some("assets/shaders/battle_background.fs"));

    let font = rl
        .load_font(&rt, "assets/fonts/Font.fnt")
        .expect("could not load font");

    let camera = Camera2D {
        target: Vector2::new(320.0 / 2.0, 240.0 / 2.0),
        offset: Vector2::new(320.0, 240.0),
        zoom: 2.0,
        rotation: 0.0,
    };

    music.set_volume(0.5);
    music.looping = true;
    // music.play_stream();

    test::print_complete_statistics(&DEFAULT_SET);

    let mut rng = rand::rng();

    let mut state = AttackInterface::new(DEFAULT_SET);
    let mut frame_count = 0;

    while !rl.window_should_close() {
        music.update_stream();
        let mut d = rl.begin_drawing(&rt);

        let time = d.get_time();
        state.update(&d, &assets, &mut rng, time);

        let mut dd = d.begin_mode2D(camera);

        dd.clear_background(Color::WHITE);

        let battle_background_ocean = assets.get_texture("battle_background_ocean");

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

        dd.draw_texture(assets.get_texture("enemy"), 350, 50, Color::WHITE);

        dd.draw_texture(assets.get_texture("girl2"), 100, 300, Color::WHITE);
        dd.draw_texture(
            assets.get_texture("girl_torso"),
            10,
            480 - 128 + (f64::sin(time * 2.0) * 4.0).round() as i32,
            Color::WHITE,
        );

        dd.draw_texture(
            assets.get_texture("girl3"),
            140,
            480 - 128 + (f64::sin(time * 2.0 + 1.0) * 4.0).round() as i32,
            Color::WHITE,
        );

        dd.draw_text_ex(
            &font,
            "ENN",
            Vector2::new(55.0, 460.0),
            16.0,
            1.0,
            Color::WHITE,
        );

        dd.draw_text_ex(
            &font,
            "KUE",
            Vector2::new(185.0, 460.0),
            16.0,
            1.0,
            Color::WHITE,
        );

        state.draw(&mut dd, &assets, time, frame_count, &font, &mut rng);

        frame_count += 1;
    }

    Ok(())
}
