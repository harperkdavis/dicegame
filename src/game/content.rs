use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    ops::Index,
    path::PathBuf,
};

pub use dialogue::Line;
pub use room::Room;
use rust_embed::Embed;
use smartstring::{LazyCompact, SmartString};

use crate::res::Res;

pub use super::battle::{EnemyDef, ItemDef, PartyDef};

pub mod dialogue;
pub mod room;
pub mod seq;

fn cnt_id(file_path: &str) -> String {
    let mut path_buf = PathBuf::from(file_path);
    path_buf.set_extension("");
    path_buf.to_string_lossy().to_string()
}

type ContentId = SmartString<LazyCompact>;

pub trait Content: Sized {
    type Context: Copy;
    type Asset: Embed;

    fn id(file_path: &str) -> ContentId {
        ContentId::from(cnt_id(file_path))
    }

    fn load(ctx: Self::Context, res: &Res, data: &'static [u8]) -> eyre::Result<Self>;
}

pub struct Library<C: 'static>(HashMap<ContentId, C>);

impl<C: Content + 'static> Library<C> {
    fn load_one(ctx: C::Context, res: &Res, file_path: &str) -> eyre::Result<C> {
        let file = C::Asset::get(file_path).ok_or_else(|| {
            eyre::eyre!("fatal: cannot load asset at {file_path:?}: invalid path")
        })?;

        let data: &'static [u8] = match file.data {
            Cow::Borrowed(d) => d,
            Cow::Owned(d) => Box::leak(Box::new(d)),
        };

        C::load(ctx, res, data)
    }

    pub fn load(ctx: C::Context, res: &Res) -> eyre::Result<Self> {
        let inner: Result<_, _> = C::Asset::iter()
            .map(|file_path| {
                Self::load_one(ctx, res, &file_path).map(|res| (C::id(&file_path), res))
            })
            .collect();

        inner.map(|inner| Self(inner))
    }

    pub fn get<K>(&self, id: &K) -> &C
    where
        K: Eq + Hash + Debug + ?Sized,
        ContentId: Borrow<K>,
    {
        self.0
            .get(id)
            .ok_or_else(|| eyre::eyre!("fatal: invalid content key {id:?}"))
            .unwrap()
    }

    pub fn has<K>(&self, id: &K) -> bool
    where
        K: Eq + Hash + Debug + ?Sized,
        ContentId: Borrow<K>,
    {
        self.0.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ContentId, &C)> {
        self.0.iter()
    }
}

impl<C: Content + 'static, K> Index<&K> for Library<C>
where
    K: Eq + Hash + Debug + ?Sized,
    ContentId: Borrow<K>,
{
    type Output = C;

    fn index(&self, index: &K) -> &Self::Output {
        self.get(index)
    }
}

#[derive(Clone, Copy)]
pub struct Cnt {
    pub items: &'static Library<ItemDef>,
    pub party: &'static Library<PartyDef>,
    pub enemies: &'static Library<EnemyDef>,
    pub rooms: &'static Library<Room>,
}

impl Cnt {
    pub fn load(res: &Res) -> eyre::Result<Self> {
        let items: &'static _ = Box::leak(Box::new(
            Library::load((), res).map_err(|e| eyre::eyre!("failed to load all items: {e}"))?,
        ));
        let party: &'static _ =
            Box::leak(Box::new(Library::load((), res).map_err(|e| {
                eyre::eyre!("failed to load all party members: {e}")
            })?));
        let enemies: &'static _ = Box::leak(Box::new(
            Library::load(items, res)
                .map_err(|e| eyre::eyre!("failed to load all enemies: {e}"))?,
        ));
        let rooms: &'static _ = Box::leak(Box::new(
            Library::load((), res).map_err(|e| eyre::eyre!("failed to load all rooms: {e}"))?,
        ));

        Ok(Self {
            items,
            party,
            enemies,
            rooms,
        })
    }
}
