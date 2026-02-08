pub mod battle;
pub mod content;

use std::cell::RefCell;
use std::rc::Rc;

use content::Cnt;
pub use content::room::{self, Room};
pub use content::{Content, dialogue};

use crate::res::Res;

#[derive(Clone, Copy)]
pub struct Game<'a> {
    pub res: &'a Res<'a>,
    pub cnt: Cnt,
}
