use rand::{Rng, seq::IndexedRandom};
use rust_embed::Embed;
use serde::Deserialize;

use crate::{
    Str,
    game::{Content, content::Library},
};

use super::ItemDef;

#[derive(Deserialize)]
pub struct Drop {
    item: Option<String>,
    weight: u32,
    value: u32,
}

pub fn calculate_drops(drop_total: u32, drops: &[Drop], rng: &mut impl Rng) -> Vec<Str> {
    let mut out = Vec::new();
    let mut current_value = 0;

    while let Ok(current) = drops.choose_weighted(rng, |f| f.weight) {
        if let Some(id) = current.item.as_ref() {
            out.push(id.into());
        }
        current_value += current.value;
        if current_value >= drop_total {
            break;
        }
    }

    out
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

    // Money rewarded for defeating this enemy.
    pub reward: u32,
    // Amount to randomize reward, defaults to 10% of the reward
    pub reward_random: Option<i32>,
    // Total value of drops to be rewarded
    pub drop_total: u32,
    // Amount to randomize drop total, defaults to zero.
    pub drop_total_random: Option<i32>,

    // Drop table
    pub drops: Vec<Drop>,

    // Flavor text
    pub flavor: Option<Vec<String>>,
    pub flavor_intro: Option<Vec<String>>,
    pub flavor_defeat: Option<Vec<String>>,
}

impl EnemyDef {
    pub fn calculate_reward(&self, rng: &mut impl Rng) -> u32 {
        let rr = self.reward_random.unwrap_or((self.reward / 10) as i32);
        self.reward
            .saturating_add_signed(rng.random_range(-rr..=rr))
    }

    pub fn calculate_drops(&self, rng: &mut impl Rng) -> Vec<Str> {
        let dtr = self.drop_total_random.unwrap_or(0);
        let drop_total = self
            .drop_total
            .saturating_add_signed(rng.random_range(-dtr..=dtr));

        calculate_drops(drop_total, &self.drops, rng)
    }

    fn select_random_flavor<'a>(f: Option<&'a Vec<String>>, rng: &mut impl Rng) -> Option<&'a str> {
        f.and_then(|f| f.choose(rng)).map(|s| s.as_str())
    }

    pub fn generate_flavor(&self, rng: &mut impl Rng) -> Option<&str> {
        Self::select_random_flavor(self.flavor.as_ref(), rng)
    }

    pub fn generate_flavor_intro(&self, rng: &mut impl Rng) -> Option<&str> {
        Self::select_random_flavor(self.flavor_intro.as_ref(), rng)
    }

    pub fn generate_flavor_defeat(&self, rng: &mut impl Rng) -> Option<&str> {
        Self::select_random_flavor(self.flavor_defeat.as_ref(), rng)
    }
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
