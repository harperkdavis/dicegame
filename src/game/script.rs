use mlua::{ExternalResult, Lua};

use crate::game::content::{
    dialogue,
    seq::{self, SeqDef},
};

pub fn create_context<'a>() -> mlua::Result<Lua> {
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
    "#;

    lua.load(bootstrap).exec()?;

    Ok(lua)
}

pub fn load_sequence(room: &str, seq_def: &SeqDef, lua: &Lua) -> mlua::Result<mlua::Function> {
    let lua_code = seq_def.into_lua_code(room);
    lua.load(lua_code).into_function()
}
