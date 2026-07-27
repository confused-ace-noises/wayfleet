use std::ops::{Deref, DerefMut};

use smithay::utils::{Logical, Size};

use crate::layout::WayfleetWindow;

#[derive(Debug)]
pub struct Tile {
    pub window: WayfleetWindow,
    pub size: Size<i32, Logical>
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window
    }
}

impl Deref for Tile {
    type Target = WayfleetWindow;
    
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DerefMut for Tile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}