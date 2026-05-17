use crate::game::battle::PartyDef;

pub const DEFEND_LINES: [&str; 5] = [
    "%name% braces for impact!",
    "%name% holds %possessive% ground!",
    "%name% plants %possessive% feet!",
    "%name% prepares %reflexive% for an attack!",
    "%name% entrenches %possessive% position!",
];

pub fn replace_placeholders(text: &str, def: &'static PartyDef) -> String {
    text.replace("%name%", &def.name)
        .replace("%personal%", &def.pronoun_personal)
        .replace("%possessive%", &def.pronoun_possessive)
        .replace("%reflexive%", &def.pronoun_reflexive)
}
