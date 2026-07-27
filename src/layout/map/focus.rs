use smithay::desktop::Space;

use crate::layout::{WayfleetWindow, map::{coordinate::Coordinate, focus::ShiftFocusOutput::Invalid, tile::TileType}};

use super::{Map, coordinate::Direction};

pub enum ShiftFocusOutput {
    Success(WayfleetWindow),
    Invalid,
    OutOfBounds,
    OutOfBoundsHinted(i32),
}

impl Map {
    pub fn new_focus_at(&mut self, loc: Coordinate, space: &Space<WayfleetWindow>) -> bool {
        if self.is_valid_coord(loc) {
            let loc = match self[&loc].as_ref().unwrap().tile_type {
                TileType::Leader { coord, .. } => coord,
                TileType::Regular(coordinate) => coordinate,
            };

            self.focus = Some(loc);

            if let Err(to_shift) = self.is_visible(loc) {
                self.shift_all(to_shift, space);
            }
            true
        } else {
            false
        }
    }

    pub fn new_focus(&mut self, window: &WayfleetWindow, space: &Space<WayfleetWindow>) -> bool {
        self.search_tile(window).is_some_and(|coord| self.new_focus_at(coord, space))
    }

    pub fn shift_focus(&mut self, direction: Direction, space: &Space<WayfleetWindow>) -> ShiftFocusOutput {

        let Some(current_focus) = self.focus else { return Invalid };

        let Some(tile) = self[&current_focus].as_ref() else { return Invalid };

        let super::tile::Tile { tile_type: TileType::Leader { cols, .. }, .. } = self.get_leader(tile) else { unreachable!() }; 

        let mut checked_coord = tile.find_outskirts(self, &direction)[0];

        loop {
            checked_coord = checked_coord.step_towards(direction);

            if !self.is_valid_coord(checked_coord) {
                // went out of bounds

                if checked_coord.row == -1 {
                    // went out of bounds searching vertically, the focus needs to pass over
                    // to the privileged strip
                    let pos = self.get_position_shifted(checked_coord);

                    // TODO: figure out if we need to reset the the focused position to None
                    
                    // TODO: fix that it doesn't take into account the in-between window spaces that are
                    // occupied by winows that are larger than 0,0
                    return ShiftFocusOutput::OutOfBoundsHinted(pos.x + ((*cols as i32 + 1) * self.cell_width) / 2);
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
                    super::tile::TileType::Regular(coordinate)  => coordinate,
                };

                let window = new_tile.window.clone();

                self.new_focus_at(leader_coord, space);
                return ShiftFocusOutput::Success(window);
            }
            // else just continue
        }
    }

    pub fn new_focus_hinted(&mut self, hint: i32, space: &Space<WayfleetWindow>) -> Option<WayfleetWindow> {
        let mut point = self.offset;
        point.x = hint;
        point.y += 1;

        let (window, _) = space.element_under(point.to_f64())?;

        self.new_focus(window, space).then_some(window.clone())
    }
}