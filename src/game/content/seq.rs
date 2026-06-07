use std::hash::{DefaultHasher, Hash, Hasher};

use mlua::UserData;
use serde::{Deserialize, Serialize};

use crate::{Str, game::content::Line};

#[derive(Clone, Debug)]
pub enum Event {
    // blocking events, requries player to progress...
    Write(Vec<Line>),
    Choice(Vec<String>),
    Wait(f32),

    GetFlag(String),
    SetFlag(String, i64),

    SetMusic(Option<Str>),
    PlaySound(Str),
    PlaySoundAndWait(Str, Option<f32>),

    SetDirection(i64),
}

impl UserData for Event {}

#[derive(Deserialize, Serialize, Hash, Clone)]
#[serde(untagged)]
pub enum SeqDef {
    Simple(Box<[String]>),
    Complex(String),
}

pub fn create_line(line: &str) -> String {
    format!("write([[\n{line}\n]])")
}

// Create a sequence in which interacting multiple times advances a set of dialogue options.
// The flag to handle this is dynamically generated.
pub fn create_multi_interact(parts: &[String], flag: &str) -> String {
    let mut pre = format!("local f = flag(\"{flag}\")");
    let post = parts
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let is_first = i == 0;
            let is_last = i == parts.len() - 1;
            let line = create_line(s.as_str());

            if is_first {
                format!("if not f or f == 0 then\n  {line}\n  set_flag(\"{flag}\", 1)")
            } else if is_last {
                format!("else\n{line}\nend")
            } else {
                format!(
                    "elseif f == {i} then\n  {line}\n  set_flag(\"{flag}\", {})",
                    i + 1
                )
            }
        })
        .collect::<Box<[_]>>()
        .join("\n");

    pre.push_str(&post);

    pre
}

impl SeqDef {
    pub fn content_hash(&self) -> u64 {
        let mut dh = DefaultHasher::new();
        self.hash(&mut dh);
        dh.finish()
    }

    pub fn unique_id(&self, room: &str) -> String {
        format!("seq/{room}/{}", self.content_hash())
    }

    pub fn requires_flag(&self) -> bool {
        matches!(self, SeqDef::Simple(v) if v.len() > 1)
    }

    pub fn conversation_flag(&self, room: &str) -> Option<String> {
        self.requires_flag()
            .then(|| format!("{}/conv", self.unique_id(room)))
    }

    pub fn get_lua_code(&self, room: &str) -> String {
        match self {
            Self::Simple(v) => match &v[..] {
                [] => "".to_string(),
                [one] => create_line(one.as_str()),
                many => create_multi_interact(many, &self.conversation_flag(room).unwrap()),
            },
            Self::Complex(s) => s.replace("%namespace%", &self.unique_id(room)).to_owned(),
        }
    }
}
