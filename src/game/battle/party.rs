use rust_embed::Embed;
use serde::Deserialize;

use crate::game::Content;

#[derive(Deserialize)]
pub struct PartyDef {
    pub name: String,

    pub sprite: String,
    pub sprite_battle: String,

    pub pronoun_personal: String,
    pub pronoun_possessive: String,
    pub pronoun_reflexive: String,
}

#[derive(Embed)]
#[folder = "cnt/party"]
pub struct PartyDefAsset;

impl Content for PartyDef {
    type Context = ();
    type Asset = PartyDefAsset;

    fn load(_: Self::Context, res: &crate::res::Res, data: &'static [u8]) -> eyre::Result<Self> {
        let def: Self = toml_edit::de::from_slice(data)
            .map_err(|e| eyre::eyre!("failed to deserialize item: {e}"))?;

        // will crash if sprite is not found during loading rather than runtime.
        res.tex(def.sprite.as_str());
        res.tex(def.sprite_battle.as_str());

        Ok(def)
    }
}
