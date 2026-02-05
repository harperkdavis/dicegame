use std::{array, ops::Index};

use super::{DICE_COUNT, Die, RollResult};

#[derive(Clone, Copy, Debug)]
pub struct DiceSet([Die; DICE_COUNT]);

impl DiceSet {
    pub const fn new(inner: [Die; DICE_COUNT]) -> Self {
        Self(inner)
    }
}

impl Index<usize> for DiceSet {
    type Output = Die;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index.min(5)]
    }
}

pub struct DiceSetIter<'a> {
    dice_set: &'a DiceSet,
    indices: [usize; DICE_COUNT],
    done: bool,
}

impl<'a> Iterator for DiceSetIter<'a> {
    type Item = RollResult;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let roll_result = array::from_fn(|i| self.dice_set[i].face(self.indices[i]));

        let mut carry = true;
        for i in 0..DICE_COUNT {
            if carry {
                self.indices[i] += 1;
            }
            if self.indices[i] >= 6 {
                self.indices[i] = 0;
                carry = true;
            } else {
                carry = false;
            }
        }

        if carry {
            self.done = true;
        }

        Some(roll_result)
    }
}

impl<'a> IntoIterator for &'a DiceSet {
    type Item = RollResult;
    type IntoIter = DiceSetIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        DiceSetIter {
            dice_set: self,
            indices: [0; DICE_COUNT],
            done: false,
        }
    }
}
