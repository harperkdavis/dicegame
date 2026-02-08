use rust_embed::Embed;
use serde::Deserialize;

use crate::game::Content;

#[derive(Deserialize)]
pub struct ItemDef {
    pub name: String,
    pub short: String,
    pub description: String,
    pub sprite: String,
}

#[derive(Embed)]
#[folder = "cnt/items"]
pub struct ItemDefAsset;

impl Content for ItemDef {
    type Context = ();
    type Asset = ItemDefAsset;

    fn load(_: Self::Context, res: &crate::res::Res, data: &'static [u8]) -> eyre::Result<Self> {
        let item_def: Self =
            toml::from_slice(data).map_err(|e| eyre::eyre!("failed to deserialize item: {e}"))?;

        // will crash if sprite is not found during loading rather than runtime.
        res.tex(item_def.sprite.as_str());

        Ok(item_def)
    }
}
