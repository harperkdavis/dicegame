use rust_embed::Embed;
use serde::Deserialize;

use crate::game::{Content, content::Library};

use super::ItemDef;

#[derive(Deserialize)]
pub struct Drop {
    item: Option<String>,
    weight: u32,
    value: u32,
}

#[derive(Deserialize)]
pub struct EnemyDef {
    pub name: String,
    pub sprite: String,

    // Total HP
    pub health: u32,
    // Flat damage reduction
    pub defense: u32,
    // Amount of damage dealt when attacking
    pub attack: u32,

    // Drop table
    pub drops: Vec<Drop>,
}

#[derive(Embed)]
#[folder = "cnt/enemies"]
pub struct EnemyAsset;

impl Content for EnemyDef {
    type Context = &'static Library<ItemDef>;
    type Asset = EnemyAsset;

    fn load(ctx: Self::Context, res: &crate::res::Res, data: &'static [u8]) -> eyre::Result<Self> {
        let enemy_def: Self =
            toml::from_slice(data).map_err(|e| eyre::eyre!("failed to deserialize item: {e}"))?;

        // panic if texture is not valid
        res.tex(enemy_def.sprite.as_str());

        for drop in &enemy_def.drops {
            if let Some(item) = &drop.item {
                // panic if item is not valid
                ctx.get(item.as_str());
            }
        }

        Ok(enemy_def)
    }
}
