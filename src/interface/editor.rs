use std::fs;

use raylib::{
    camera::Camera2D,
    color::Color,
    ffi::{KeyboardKey, MouseButton},
    math::{Rectangle, Vector2},
    prelude::{RaylibDraw, RaylibDrawHandle, RaylibMode2DExt},
};
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::{
    Str,
    game::{
        Frame, Static,
        content::{
            Room,
            room::{Layer, LayerItem, Object},
        },
    },
    util::TextOutline,
};

const MAX_RESULTS: usize = 27;
const GRID_SIZE_I: i32 = 20;
const GRID_SIZE: f32 = 20.0;

const COMMAND_NEW_ROOM: &str = "n";
const COMMAND_LOAD_ROOM: &str = "l";
const COMMAND_LOAD_TEXTURE: &str = "t";
const COMMAND_SET_BACKGROUND: &str = "bg";
const COMMAND_SET_MUSIC: &str = "mus";

#[derive(Clone, Copy, IntoStaticStr, PartialEq, Eq)]
#[repr(usize)]
enum Mode {
    Normal,
    Add,
    MultiAdd,
    Translate,
    Scale,
}

#[derive(Default)]
struct Palette {
    texture: Option<Str>,
}

struct RoomEditor {
    id: String,
    room: Room,

    camera_x: f32,
    camera_y: f32,

    result: Result<String, String>,
    layer: Layer,
    mode: Mode,

    grid: bool,
    using_grid: bool,

    mouse_pos: (i32, i32),
    start_pos: Option<(i32, i32)>,

    selected: Option<LayerItem>,
    hover: Option<LayerItem>,
}

impl RoomEditor {
    pub fn new(id: &str, room: Room) -> Self {
        Self {
            id: id.to_owned(),
            room,

            camera_x: 0.0,
            camera_y: 0.0,

            layer: Layer::Object,
            mode: Mode::Normal,
            result: Ok(format!("loaded {id}.")),

            grid: true,
            using_grid: true,

            mouse_pos: (0, 0),
            start_pos: None,

            selected: None,
            hover: None,
        }
    }

    pub fn new_empty(id: &str) -> Self {
        Self::new(id, Room::default())
    }

    pub fn load(id: &str, s: Static) -> Self {
        let room = s.room(id);
        Self::new(id, room.to_owned())
    }

    pub fn get_camera(&self) -> Camera2D {
        Camera2D {
            offset: Vector2::new(640.0 / 2.0, 480.0 / 2.0),
            target: Vector2::new(self.camera_x.floor(), self.camera_y.floor()),
            zoom: 1.0,
            rotation: 0.0,
        }
    }

    pub fn set_background(&mut self, bg: Str) {
        self.room.background = Some(bg);
    }

    pub fn set_music(&mut self, mus: Str) {
        self.room.music = Some(mus);
    }

    pub fn using_grid(&self, alt: bool) -> bool {
        self.grid ^ alt
    }

    fn get_world_position(&self, mx: i32, my: i32, sc_w: i32, sc_h: i32, grid: bool) -> (i32, i32) {
        let raw_world = (
            (mx * 640 / sc_w) + self.camera_x as i32 - 640 / 2,
            (my * 480 / sc_h) + self.camera_y as i32 - 480 / 2,
        );
        if grid {
            (
                (raw_world.0 / GRID_SIZE_I) * GRID_SIZE_I,
                (raw_world.1 / GRID_SIZE_I) * GRID_SIZE_I,
            )
        } else {
            raw_world
        }
    }

    fn get_place_pos(mouse: (i32, i32), width: i32, height: i32) -> (i32, i32) {
        (mouse.0 - width / 2, mouse.1 - height)
    }

    fn get_hover(room: &Room, mx: i32, my: i32, current_layer: Layer) -> Option<LayerItem> {
        let mut candidates = room
            .layout
            .overlapping_objects(mx, my)
            .map(|item| LayerItem {
                layer: Layer::Object,
                item,
            })
            .chain(
                room.layout
                    .overlapping_collision(mx, my)
                    .map(|item| LayerItem {
                        layer: Layer::Collision,
                        item,
                    }),
            )
            .chain(
                room.layout
                    .overlapping_triggers(mx, my)
                    .map(|item| LayerItem {
                        layer: Layer::Trigger,
                        item,
                    }),
            )
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            None
        } else if candidates.len() == 1 {
            Some(candidates[0])
        } else {
            // prioritize items on current layer
            candidates.sort_by(|a, b| {
                let a_num = (a.layer != current_layer) as u8;
                let b_num = (b.layer != current_layer) as u8;
                a_num.cmp(&b_num)
            });

            Some(candidates[0])
        }
    }

    pub fn update(&mut self, d: &RaylibDrawHandle, palette: &mut Palette, s: Static, frame: Frame) {
        // make sure we aren't adding an invalid texture
        if matches!(self.mode, Mode::MultiAdd | Mode::Add) && palette.texture.is_none() {
            self.mode = Mode::Normal;
        }

        self.using_grid = self.using_grid(
            d.is_key_down(KeyboardKey::KEY_LEFT_ALT) || d.is_key_down(KeyboardKey::KEY_RIGHT_ALT),
        );
        self.mouse_pos = self.get_world_position(
            d.get_mouse_x(),
            d.get_mouse_y(),
            d.get_screen_width(),
            d.get_screen_height(),
            self.using_grid,
        );
        let raw = self.get_world_position(
            d.get_mouse_x(),
            d.get_mouse_y(),
            d.get_screen_width(),
            d.get_screen_height(),
            false,
        );

        self.hover = if self.mode == Mode::Normal {
            Self::get_hover(&self.room, raw.0, raw.1, self.layer)
        } else {
            None
        };

        let is_shift = d.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
            || d.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        // ESC: clear mode and clear any transformations
        if d.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.mode = Mode::Normal;
            self.start_pos = None;
            self.selected = None;
            self.room.re_sort_y();
        }
        // x: delete selected
        if d.is_key_pressed(KeyboardKey::KEY_X)
            && self.mode == Mode::Normal
            && let Some(selected) = self.selected
        {
            // TODO: remove
            self.room.remove(selected);
            self.start_pos = None;
            self.selected = None;
        }
        // a: enter add mode (allow placing texture)
        if d.is_key_pressed(KeyboardKey::KEY_A) {
            if palette.texture.is_some() {
                self.mode = Mode::Add;
            } else {
                self.result = Err("add error: no texture selected!".to_string());
            }
        }
        // i: enter multi-add mode (allow placing multiple of whatever it is)
        if d.is_key_pressed(KeyboardKey::KEY_I) {
            if palette.texture.is_some() {
                self.mode = Mode::MultiAdd;
            } else {
                self.result = Err("add error: no texture selected!".to_string());
            }
        }

        if d.is_key_pressed(KeyboardKey::KEY_G) {
            if is_shift {
                self.grid = !self.grid;
            } else if self.selected.is_some() {
                // TODO: translate
                self.mode = Mode::Translate;
                self.start_pos = Some(self.mouse_pos);
            }
        }

        // o: switch to object layer
        if d.is_key_pressed(KeyboardKey::KEY_O) {
            self.layer = Layer::Object;
            if is_shift && let Some(selected) = self.selected {
                // auto-object
            }
        }
        // c: switch to collision layer
        if d.is_key_pressed(KeyboardKey::KEY_C) {
            self.layer = Layer::Collision;
            if is_shift
                && let Some(li) = self.selected
                && li.layer != Layer::Collision
            {
                // auto-collision
                let rect = match li.layer {
                    Layer::Object => self.room.layout.objects[li.item].r.auto_collision(),
                    Layer::Trigger => self.room.layout.triggers[li.item].r,
                    _ => unreachable!(),
                };

                self.selected = Some(LayerItem {
                    layer: Layer::Collision,
                    item: self.room.add_collision(rect),
                });
            }
        }
        // t: switch to trigger layer
        if d.is_key_pressed(KeyboardKey::KEY_T) {
            self.layer = Layer::Trigger;
            if is_shift
                && let Some(li) = self.selected
                && li.layer != Layer::Trigger
            {
                // auto-collision
                let rect = match li.layer {
                    Layer::Object => self.room.layout.objects[li.item].r.auto_collision(),
                    Layer::Collision => self.room.layout.collision[li.item].r,
                    _ => unreachable!(),
                };

                self.selected = Some(LayerItem {
                    layer: Layer::Trigger,
                    item: self.room.add_trigger(rect),
                });
            }
        }

        if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            // depends on mode first
            match self.mode {
                Mode::Normal => {
                    if let Some(hover) = self.hover {
                        self.selected = Some(hover);
                    } else {
                        self.selected = None;
                        self.room.re_sort_y();
                    }
                }
                Mode::MultiAdd | Mode::Add => {
                    let added = match self.layer {
                        Layer::Object => {
                            let sprite = palette.texture.clone().unwrap();
                            let tex = s.tex(&sprite);
                            Some(self.room.add_object(Object::new_from_texture(
                                Self::get_place_pos(self.mouse_pos, tex.width, tex.height),
                                sprite,
                                s,
                            )))
                        }
                        _ => None,
                    };

                    if let Some(added) = added {
                        self.selected = Some(LayerItem {
                            layer: self.layer,
                            item: added,
                        });
                        // add mode automatically deselects
                        if self.mode == Mode::Add {
                            self.mode = Mode::Normal;
                        }
                    }
                }
                Mode::Translate => {}
                _ => (),
            }
        }

        // 0: move camera back to center
        if d.is_key_pressed(KeyboardKey::KEY_ZERO) {
            self.result = Ok("reset camera.".to_string());
            self.camera_x = 0.0;
            self.camera_y = 0.0;
        }

        self.camera_x += frame.input_x * frame.delta * 1000.0;
        self.camera_y += frame.input_y * frame.delta * 1000.0;
    }

    pub fn draw(&self, d: &mut impl RaylibDraw, palette: &Palette, s: Static, frame: Frame) {
        let editor_font = s.fnt("nokia_15");

        let cam = self.get_camera();
        let mut dd = d.begin_mode2D(cam);
        self.room.draw_background(&mut dd, s);

        drop(dd);

        let grid_start_x = (-self.camera_x).rem_euclid(GRID_SIZE);
        let grid_start_y = (-self.camera_y).rem_euclid(GRID_SIZE);

        if self.grid || self.using_grid {
            let grid_alpha = if self.using_grid != self.grid {
                0.05
            } else {
                0.2
            };
            for i in -1..=(640 / GRID_SIZE_I) {
                let x = grid_start_x as i32 + i * GRID_SIZE_I;
                d.draw_line(x, 0, x, 480, Color::BLUE.alpha(grid_alpha));
            }

            for i in -1..=(640 / GRID_SIZE_I) {
                let y = grid_start_y as i32 + i * GRID_SIZE_I;
                d.draw_line(0, y, 640, y, Color::BLUE.alpha(grid_alpha));
            }
        }

        let mut dd = d.begin_mode2D(cam);
        self.room
            .draw(&mut dd, s, Some((self.hover, self.selected)));

        match self.mode {
            // draw preview
            Mode::MultiAdd | Mode::Add => match self.layer {
                Layer::Object => {
                    // texture is known to be non-null at this point.
                    let tex = s.tex(palette.texture.as_ref().unwrap());
                    let pos = Self::get_place_pos(self.mouse_pos, tex.width, tex.height);
                    dd.draw_texture(tex, pos.0, pos.1, Color::WHITE.alpha(0.5));
                    dd.draw_text_outline(
                        editor_font,
                        &format!("{pos:?}"),
                        (self.mouse_pos.0 + 10) as f32,
                        (self.mouse_pos.1 + 10) as f32,
                        Color::WHITE,
                        Color::BLACK,
                    );
                }
                _ => (),
            },
            Mode::Translate => {
                let li = self.selected.unwrap();
                let mut rect = match li.layer {
                    Layer::Object => self.room.layout.objects[li.item].r,
                    Layer::Collision => self.room.layout.collision[li.item].r,
                    Layer::Trigger => self.room.layout.triggers[li.item].r,
                };

                if li.layer == Layer::Object {
                    let tex = s.tex(&self.room.layout.objects[li.item].sprite);
                    (rect.x, rect.y) = Self::get_place_pos(self.mouse_pos, tex.width, tex.height);
                    dd.draw_texture_pro(
                        tex,
                        Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32),
                        Rectangle::from(rect),
                        Vector2::zero(),
                        0.0,
                        Color::WHITE.alpha(0.5),
                    );
                } else {
                    rect.x = self.mouse_pos.0;
                    rect.y = self.mouse_pos.1;
                }

                dd.draw_rectangle_lines(
                    rect.x,
                    rect.y,
                    u32::from(rect.w) as i32,
                    u32::from(rect.h) as i32,
                    Color::RED,
                );
            }
            _ => (),
        }

        drop(dd);

        d.draw_rectangle(
            480,
            0,
            80,
            20,
            match self.layer {
                Layer::Object => Color::DARKMAGENTA,
                Layer::Collision => Color::GREEN,
                Layer::Trigger => Color::DARKBLUE,
            },
        );
        d.draw_text_outline(
            editor_font,
            self.layer.into(),
            483.0,
            3.0,
            Color::WHITE,
            Color::BLACK,
        );
        d.draw_rectangle(
            560,
            0,
            80,
            20,
            match self.mode {
                Mode::Normal => Color::LIME,
                Mode::Add => Color::BLUE,
                Mode::MultiAdd => Color::CYAN,
                Mode::Translate => Color::DARKRED,
                Mode::Scale => Color::PURPLE,
            },
        );
        d.draw_text_outline(
            editor_font,
            self.mode.into(),
            563.0,
            3.0,
            Color::WHITE,
            Color::BLACK,
        );
        let (result_msg, result_col) = match self.result.as_deref() {
            Ok(s) => (s, Color::WHITE),
            Err(s) => (s.as_ref(), Color::RED),
        };
        d.draw_text_outline(editor_font, result_msg, 3.0, 3.0, result_col, Color::BLACK);
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Persistent {
    working_on: Option<String>,
    last_backup: Option<String>,
    last_overwritten: Option<String>,
}

impl Persistent {
    const PATH: &str = "tmp/editor.toml";

    fn write(&self) -> eyre::Result<()> {
        fs::write(Self::PATH, toml::to_string(self)?)?;
        Ok(())
    }

    fn edit(&mut self, f: impl Fn(&mut Self)) -> eyre::Result<()> {
        f(self);
        self.write()?;
        Ok(())
    }

    fn load_or_default() -> eyre::Result<Self> {
        if let Some(p) = fs::read_to_string(Self::PATH)
            .ok()
            .and_then(|s| toml::from_str::<Self>(&s).ok())
        {
            Ok(p)
        } else {
            let default = Self::default();
            default.write()?;
            Ok(default)
        }
    }
}

pub struct EditorInterface {
    persistent: Persistent,

    command: Option<String>,
    validated_command: Option<Result<String, ()>>,

    command_result: Result<String, String>,

    search_results: Option<Vec<String>>,
    autocomplete_option: usize,

    room: Option<RoomEditor>,
    palette: Palette,
}

impl EditorInterface {
    pub fn new(s: Static) -> eyre::Result<Self> {
        let persistent = Persistent::load_or_default()?;
        let room = persistent
            .working_on
            .as_deref()
            .map(|id| RoomEditor::load(id, s));
        let command_result = match room.as_ref() {
            Some(re) => Ok(format!("(resumed) loaded {}", re.id)),
            None => Err("no room to resume".to_owned()),
        };

        Ok(Self {
            persistent,

            command: None,
            validated_command: None,

            command_result,
            search_results: None,
            autocomplete_option: 0,

            room,
            palette: Palette::default(),
        })
    }
}

impl EditorInterface {
    pub fn update(
        &mut self,
        d: &mut RaylibDrawHandle,
        s: Static,
        frame: Frame,
    ) -> eyre::Result<()> {
        if let Some(command) = self.command.as_mut() {
            let prev_command = command.clone();
            if let Some(c) = d.get_char_pressed() {
                command.push(c);
            }

            if (d.is_key_pressed(KeyboardKey::KEY_TAB) || d.is_key_pressed(KeyboardKey::KEY_ENTER))
                && let Some((exec, _)) = command.split_once(' ')
                && let Some(results) = self.search_results.as_ref()
                && let Some(highlighted_result) = results.get(self.autocomplete_option)
            {
                *command = format!("{exec} {highlighted_result}");
            }

            if d.is_key_pressed(KeyboardKey::KEY_ENTER) {
                self.command_result = if let Some((exec, args)) = command.split_once(' ') {
                    let args = args.trim();
                    match exec {
                        COMMAND_NEW_ROOM => {
                            if args.split_whitespace().count() > 1 {
                                Err(format!("room id includes whitespace: {args}"))
                            } else if s.cnt.rooms.has(args) {
                                Err(format!("room id already used: {args}"))
                            } else {
                                self.room = Some(RoomEditor::new_empty(args));
                                Ok(format!("new room: {args}"))
                            }
                        }
                        COMMAND_LOAD_ROOM => {
                            if s.cnt.rooms.has(args) {
                                self.room = Some(RoomEditor::load(args, s));
                                self.persistent
                                    .edit(|p| p.working_on = Some(args.to_string()))?;
                                Ok(format!("room loaded: {args}"))
                            } else {
                                Err(format!("room not found: {args}"))
                            }
                        }
                        COMMAND_LOAD_TEXTURE => {
                            if s.res.has_tex(args) {
                                self.palette.texture = Some(args.into());
                                Ok(format!("texture loaded: {args}"))
                            } else {
                                Err(format!("texture not found: {args}"))
                            }
                        }
                        COMMAND_SET_BACKGROUND => {
                            if s.res.has_tex(args) {
                                if let Some(re) = self.room.as_mut() {
                                    re.set_background(args.into());
                                }
                                Ok(format!("set background to: {args}"))
                            } else {
                                Err(format!("texture not found: {args}"))
                            }
                        }
                        COMMAND_SET_MUSIC => {
                            if s.res.has_mus(args) {
                                if let Some(re) = self.room.as_mut() {
                                    re.set_music(args.into());
                                }
                                Ok(format!("set music to: {args}"))
                            } else {
                                Err(format!("music not found: {args}"))
                            }
                        }
                        c => Err(format!("invalid command: {c}")),
                    }
                } else {
                    Err(format!(
                        "did not provide any arguments for command: {command}"
                    ))
                };
            }

            // clear if deleted all characters
            if (d.is_key_pressed(KeyboardKey::KEY_BACKSPACE) && command.pop().is_none())
                // clear if pressing esc
                || d.is_key_pressed(KeyboardKey::KEY_ESCAPE)
                // clear if running command
                || d.is_key_pressed(KeyboardKey::KEY_ENTER)
            {
                self.command = None;
                self.search_results = None;
                self.validated_command = None;
            }

            if let Some(results) = self.search_results.as_ref()
                && !results.is_empty()
            {
                if d.is_key_pressed(KeyboardKey::KEY_DOWN) {
                    self.autocomplete_option += 1;
                } else if d.is_key_pressed(KeyboardKey::KEY_UP) {
                    self.autocomplete_option = self.autocomplete_option.saturating_sub(1);
                }
                self.autocomplete_option = self.autocomplete_option.clamp(0, results.len())
            }

            if Some(prev_command) != self.command
                && let Some(new_command) = self.command.as_mut()
            {
                if let Some((exec, args)) = new_command.split_once(' ') {
                    let results = match exec {
                        COMMAND_LOAD_ROOM => {
                            self.validated_command = Some(Ok(COMMAND_LOAD_ROOM.to_owned()));

                            Some(
                                s.cnt
                                    .rooms
                                    .iter()
                                    .filter_map(|(k, _)| k.contains(args).then_some(k.to_string()))
                                    .take(MAX_RESULTS)
                                    .collect::<Vec<_>>(),
                            )
                        }
                        COMMAND_SET_MUSIC => {
                            self.validated_command = Some(Ok(COMMAND_SET_MUSIC.to_owned()));

                            Some(
                                s.res
                                    .mus_iter()
                                    .filter_map(|(k, _)| k.contains(args).then_some(k.to_string()))
                                    .take(MAX_RESULTS)
                                    .collect::<Vec<_>>(),
                            )
                        }
                        COMMAND_LOAD_TEXTURE | COMMAND_SET_BACKGROUND => {
                            self.validated_command = Some(Ok(exec.to_owned()));

                            Some(
                                s.res
                                    .tex_iter()
                                    .filter_map(|(k, _)| k.contains(args).then_some(k.to_string()))
                                    .take(MAX_RESULTS)
                                    .collect::<Vec<_>>(),
                            )
                        }
                        COMMAND_NEW_ROOM => {
                            self.validated_command = Some(Ok(exec.to_owned()));
                            None
                        }
                        _ => {
                            self.validated_command = Some(Err(()));
                            None
                        }
                    };
                    if let Some(results) = results {
                        if results.is_empty() {
                            self.autocomplete_option = 0;
                        } else if self.autocomplete_option >= results.len() {
                            self.autocomplete_option = results.len() - 1;
                        }

                        self.search_results = Some(results);
                    } else {
                        self.search_results = None;
                    }
                } else {
                    self.validated_command = None;
                    self.search_results = None;
                }
            }
        } else if d.is_key_pressed(KeyboardKey::KEY_SLASH) {
            self.command = Some("t ".to_string());
            self.autocomplete_option = 0;
        } else if d.get_char_pressed() == Some(':') {
            self.command = Some(String::new());
            self.autocomplete_option = 0;
        } else if let Some(room_editor) = self.room.as_mut() {
            room_editor.update(d, &mut self.palette, s, frame);
        }

        Ok(())
    }

    pub fn draw(&self, d: &mut impl RaylibDraw, s: Static, frame: Frame) {
        let editor_font = s.fnt("nokia_15");
        if let Some(re) = self.room.as_ref() {
            re.draw(d, &self.palette, s, frame);
        } else {
            d.draw_text("(no room loaded)", 100, 100, 15, Color::GRAY);
        }

        if let Some(command) = self.command.as_deref() {
            d.draw_text_outline(
                editor_font,
                &format!(
                    ":{command}{}",
                    if frame.time % 0.5 < 0.25 { '_' } else { ' ' }
                ),
                1.0,
                462.0,
                if let Some(r) = self.validated_command.as_ref() {
                    if r.is_ok() { Color::WHITE } else { Color::RED }
                } else {
                    Color::GRAY
                },
                Color::BLACK,
            );

            if let Some(results) = self.search_results.as_deref() {
                d.draw_rectangle(0, 0, 640, 460, Color::new(0, 0, 0, 127));
                if results.is_empty() {
                    d.draw_text_outline(
                        editor_font,
                        "(no results)",
                        2.0,
                        2.0,
                        Color::RED,
                        Color::BLACK,
                    );
                }
                for (i, room_name) in results.iter().enumerate() {
                    if i == self.autocomplete_option {
                        d.draw_rectangle(1, 1 + i as i32 * 17, 638, 17, Color::WHITE);
                    }
                    d.draw_text_outline(
                        editor_font,
                        room_name,
                        3.0,
                        2.0 + i as f32 * 17.0,
                        Color::WHITE,
                        Color::BLACK,
                    );
                }
            }
        } else {
            let msg = match self.command_result.as_deref() {
                Ok(s) => s,
                Err(s) => s,
            };
            d.draw_text_outline(
                editor_font,
                &format!(".{msg}",),
                1.0,
                462.0,
                if self.command_result.is_ok() {
                    Color::GRAY
                } else {
                    Color::RED
                },
                Color::BLACK,
            );
        }
    }
}
