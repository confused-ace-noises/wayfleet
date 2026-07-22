use smithay::desktop::{Space, Window};

use crate::layout::map::focus::ShiftFocusOutput::Invalid;

use super::{Map, coordinate::Direction};

pub enum ShiftFocusOutput {
    Success(Window),
    Invalid,
    OutOfBounds,
    OutOfBoundsHinted(i32),
}

impl Map {
    pub fn new_focus(&mut self, window: &Window) -> bool {
        let location = self.search_tile(window);

        self.focus = location;

        self.focus.is_some()
    }

    pub fn shift_focus(&mut self, direction: Direction) -> ShiftFocusOutput {

        let Some(current_focus) = self.focus else { return Invalid };

        let Some(tile) = self[&current_focus].as_ref() else { return Invalid };

        let mut checked_coord = tile.find_outskirts(self, &direction)[0];

        loop {
            checked_coord = checked_coord.step_towards(direction);

            if !self.is_valid_coord(checked_coord) {
                // went out of bounds

                if checked_coord.row == -1 {
                    // went out of bounds searching vertically, the focus needs to pass over
                    // to the privileged strip
                    let pos = self.get_position(checked_coord);

                    // TODO: figure out if we need to reset the the focused position to None
                    return ShiftFocusOutput::OutOfBoundsHinted(pos.x);
                } else {
                    return ShiftFocusOutput::OutOfBounds;
                }
            }

            let Some(new_tile) = &self[&checked_coord] 
            else {
                // empty tile, continue
                continue; 
            };

            if new_tile.window != tile.window {
                // new window!
                let leader_coord = match new_tile.tile_type {
                    super::tile::TileType::Leader { coord, .. } => coord,
                    super::tile::TileType::Regular(coordinate) => coordinate,
                };

                let window = new_tile.window.clone();

                self.focus = Some(leader_coord);
                return ShiftFocusOutput::Success(window);
            }
            // else just continue
        }
    }

    pub fn new_focus_hinted(&mut self, hint: i32, space: &Space<Window>) -> Option<Window> {
        let mut point = self.offset;
        point.x = hint;
        point.y += 1;

        let (window, _) = space.element_under(point.to_f64())?;

        self.new_focus(window).then_some(window.clone())
    }
}