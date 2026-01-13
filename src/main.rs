mod dice;

use raylib::prelude::*;

use crate::dice::{DEFAULT_SET, result_rectangles, roll_set, score_faces};

fn main() {
    let (mut rl, rt) = raylib::init().size(640, 480).title("Hello, World").build();

    let font = rl
        .load_font(&rt, "assets/fonts/normal.fnt")
        .expect("could not load font");

    let dice = rl
        .load_texture(&rt, "assets/textures/dice.png")
        .expect("could not load texture");

    let dice_border = rl
        .load_texture(&rt, "assets/textures/dice_border.png")
        .expect("could not load texture");

    let camera = Camera2D {
        target: Vector2::new(320.0 / 2.0, 240.0 / 2.0),
        offset: Vector2::new(320.0, 240.0),
        zoom: 2.0,
        rotation: 0.0,
    };

    let mut rng = rand::rng();

    let dice_set = DEFAULT_SET;
    let mut results = roll_set(&dice_set, &mut rng);
    let mut score = score_faces(&results);

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&rt);

        let mut dd = d.begin_mode2D(camera);

        if dd.is_key_pressed(KeyboardKey::KEY_Z) {
            results = roll_set(&dice_set, &mut rng);
            score = score_faces(&results);
        }

        dd.clear_background(Color::WHITE);
        dd.draw_text_ex(
            &font,
            "Cosmic Wimpout",
            Vector2::new(10.0, 10.0),
            16.0,
            1.0,
            Color::BLACK,
        );

        for (i, rect) in result_rectangles(&dice_set, &results)
            .into_iter()
            .enumerate()
        {
            dd.draw_texture(
                &dice_border,
                56 + i as i32 * 40,
                56,
                if score.scoring[i] {
                    Color::YELLOW
                } else {
                    Color::WHITE
                },
            );
            dd.draw_texture_rec(
                &dice,
                rect,
                Vector2::new(60.0 + i as f32 * 40.0, 60.0),
                Color::WHITE,
            );
        }

        dd.draw_text_ex(
            &font,
            &format!(
                "points scored: {}\nmust reroll: {}",
                score.points, score.must_reroll
            ),
            Vector2::new(10.0, 200.0),
            16.0,
            1.0,
            Color::BLACK,
        );
    }
}
