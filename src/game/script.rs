use mlua::{ExternalResult, Lua};

use crate::game::content::{
    dialogue,
    seq::{self, SeqDef},
};

pub fn create_context() -> mlua::Result<Lua> {
    let lua = Lua::new();
    let globals = lua.globals();

    globals.set(
        "__make_get_flag",
        lua.create_function(|_, id: String| Ok(seq::Event::GetFlag(id)))?,
    )?;

    globals.set(
        "__make_set_flag",
        lua.create_function(|_, (id, value): (String, i64)| Ok(seq::Event::SetFlag(id, value)))?,
    )?;

    globals.set(
        "__make_write",
        lua.create_function(|_, text: String| {
            Ok(seq::Event::Write(
                dialogue::parse_lines(&text).into_lua_err()?,
            ))
        })?,
    )?;

    globals.set(
        "__make_choice",
        lua.create_function(|_, choices: Vec<String>| Ok(seq::Event::Choice(choices)))?,
    )?;

    globals.set(
        "__make_wait",
        lua.create_function(|_, seconds: f32| Ok(seq::Event::Wait(seconds)))?,
    )?;

    globals.set(
        "__make_set_music",
        lua.create_function(|_, track: Option<String>| {
            Ok(seq::Event::SetMusic(track.map(|s| s.into())))
        })?,
    )?;

    globals.set(
        "__make_play_sound",
        lua.create_function(|_, sound: String| Ok(seq::Event::PlaySound(sound.into())))?,
    )?;

    globals.set(
        "__make_play_sound_and_wait",
        lua.create_function(|_, (sound, wait_for): (String, Option<f32>)| {
            Ok(seq::Event::PlaySoundAndWait(sound.into(), wait_for))
        })?,
    )?;

    globals.set(
        "__make_set_direction",
        lua.create_function(|_, direction: i64| Ok(seq::Event::SetDirection(direction)))?,
    )?;

    let bootstrap = r#"
        function flag(id)
            return coroutine.yield(__make_get_flag(id))
        end

        function set_flag(id, value)
            return coroutine.yield(__make_set_flag(id, value))
        end

        function write(text)
            return coroutine.yield(__make_write(text))
        end

        function choice(choices)
            return coroutine.yield(__make_choice(choices))
        end

        function wait(seconds)
            return coroutine.yield(__make_wait(seconds))
        end

        function set_music(music)
            return coroutine.yield(__make_set_music(music))
        end

        function stop_music()
            set_music(nil)
        end

        function play_sound(sound)
            return coroutine.yield(__make_play_sound(sound))
        end

        function play_sound_and_wait(sound, time)
            return coroutine.yield(__make_play_sound_and_wait(sound, time))
        end

        function set_direction(dir)
            return coroutine.yield(__make_set_direction(dir))
        end
    "#;

    lua.load(bootstrap).exec()?;

    Ok(lua)
}

pub fn load_sequence(room: &str, seq_def: &SeqDef, lua: &Lua) -> mlua::Result<mlua::Function> {
    let lua_code = seq_def.get_lua_code(room);
    lua.load(lua_code).into_function()
}
