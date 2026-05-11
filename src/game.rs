pub mod battle;
pub mod content;
pub mod state;

use content::Cnt;
pub use content::Content;
use smartstring::{LazyCompact, SmartString};

use crate::res::Res;

pub type Str = SmartString<LazyCompact>;

#[derive(Clone, Copy)]
pub struct Game<'a> {
    pub res: &'a Res<'a>,
    pub cnt: Cnt,
}
