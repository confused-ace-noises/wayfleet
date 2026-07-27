use std::time::Duration;

use smithay::{
    desktop::{Space, Window},
    utils::{Logical, Point, Rectangle, Size},
};

use crate::{
    animations::{Easing, InfoType, MoveAnimation}, layout::{WayfleetWindow, map::coordinate::Direction},
};

use super::{
    Coordinate, Map,
    tile::{Tile, TileType},
};

impl Map {
    pub fn is_valid_coord(&self, coord: Coordinate) -> bool {
        coord.column < self.columns as i32
            && coord.row < self.rows as i32
            && coord.column >= 0
            && coord.row >= 0
    }

    /// NOT WORKING:
    /// window.geometry() doesn't work as i thought, need to find the window position
    /// by knowing the layout, which isn't completed yet, so
    /// TODO, FIXME
    pub fn find_window(&self, point: Point<i32, Logical>) -> Option<&WayfleetWindow> {
        for row in 0..self.rows {
            let row_rect = Rectangle::new(
                self.get_position_shifted(Coordinate {
                    row: row as i32,
                    column: 0,
                }),
                Size::new(self.total_width(), self.cell_height),
            );

            if row_rect.contains(point) {
                let row = &self.map[row];
                // found row, now find column
                for col in row.iter() {
                    if let Some(tile) = col.as_ref()
                        && tile.geometry().contains(point)
                    {
                        return Some(&tile.window);
                    }
                }

                // didn't find it here, the pointer is in some random gap
                break;
            }
        }

        None
    }

    /// NOT WORKING:
    /// window.geometry() doesn't work as i thought, need to find the window position
    /// by knowing the layout, which isn't completed yet, so
    /// TODO, FIXME
    pub fn find_window_pos(
        &self,
        point: Point<i32, Logical>,
        space: &Space<WayfleetWindow>,
    ) -> Option<(&Window, Point<i32, Logical>)> {
        for row in 0..self.rows {
            let row_rect = Rectangle::new(
                self.get_position_shifted(Coordinate {
                    row: row as i32,
                    column: 0,
                }),
                Size::new(self.total_width(), self.cell_height),
            );

            if row_rect.contains(point) {
                let row = &self.map[row];
                // found row, now find column
                for col in row.iter() {
                    if let Some(tile) = col
                        && tile.geometry().contains(point)
                    {
                        let pos = space.element_location(&tile.window).unwrap();
                        return Some((tile, pos));
                    }
                }

                // didn't find it here, the pointer is in some random gap
                break;
            }
        }

        None
    }

    pub fn get_position_raw(&self, Coordinate { row, column }: Coordinate) -> Point<i32, Logical> {
        Point::new(
            column * self.cell_width + (column) * self.spaces.horizontal as i32 + self.offset.x,
            row * self.cell_height + (row) * self.spaces.vertical as i32 + self.offset.y,
        ) + Point::new(column.signum(), row.signum())
    }

    pub fn get_position_shifted(&self, coordinate: Coordinate) -> Point<i32, Logical> {
        self.get_position_raw(coordinate) - self.viewport_offset
    }

    pub fn get_size(&self) -> Size<i32, Logical> {
        Size::new(self.cell_width, self.cell_height)
    }

    pub fn total_width(&self) -> i32 {
        self.cell_width * self.columns as i32
            + (self.columns as i32 - 1) * self.spaces.horizontal as i32
    }

    /// # Safety
    /// forceful function, could break some windows
    ///
    /// # Panics
    /// will panic if leader isn't actually a leader.
    pub unsafe fn repoint_regualr_tiles(&mut self, leader: Coordinate) {
        let Tile { window, .. } = self[&leader].as_ref().unwrap();
        unsafe { self.change_regulars(leader, Some(Tile::new_regular(window.clone(), leader))) };
    }

    /// # Safety
    /// forceful function, could break some windows
    ///
    /// # Panics
    /// will panic if leader isn't actually a leader.
    pub unsafe fn change_regulars(&mut self, leader: Coordinate, change_to: Option<Tile>) {
        let Some(Tile {
            tile_type: TileType::Leader { rows, cols, .. },
            ..
        }) = self[&leader].as_ref()
        else {
            panic!("wrong arguments passed to repoint_regular_arguments")
        };

        let last = leader
            + Coordinate {
                row: *rows as i32,
                column: *cols as i32,
            };
        let mut first = true;

        for r in leader.row..=last.row {
            for c in leader.column..=last.column {
                if first {
                    first = false;
                    continue;
                }

                self[&(r, c).into()] = change_to.clone()
            }
        }
    }

    pub fn search_tile(&self, searched_window: &WayfleetWindow) -> Option<Coordinate> {
        for r in 0..self.rows {
            for c in 0..self.columns {
                let coord = (r as i32, c as i32).into();
                if let Some(Tile { window, .. }) = &self[&coord]
                    && *window == *searched_window
                {
                    return Some(coord);
                }
            }
        }

        None
    }

    pub fn directional_search(
        &self,
        search_from: Coordinate,
        direction: Direction,
    ) -> Option<&WayfleetWindow> {
        let mut last_searched = search_from;
        let mut last_tile = self[&last_searched].as_ref();
        loop {
            last_searched = last_searched.step_towards(direction);

            if !self.is_valid_coord(last_searched) {
                break None;
            }

            let new_tile = self[&last_searched].as_ref();

            if new_tile == last_tile {
                continue;
            } else {
                last_tile = new_tile;
            }

            let Some(tile) = self[&last_searched].as_ref() else {
                continue;
            };

            break Some(&tile.window);
        }
    }

    pub fn radial_search(&self, search_from: Coordinate) -> Option<&WayfleetWindow> {
        let left = self.directional_search(search_from, Direction::Left);

        if let Some(left) = left {
            return Some(left);
        }

        let right = self.directional_search(search_from, Direction::Right);

        if let Some(right) = right {
            return Some(right);
        }

        let up = self.directional_search(search_from, Direction::Up);

        if let Some(up) = up {
            return Some(up);
        }

        let down = self.directional_search(search_from, Direction::Down);

        if let Some(down) = down {
            return Some(down);
        }

        let searching_tile = self[&search_from].as_ref();

        for row in 0..self.rows {
            for column in 0..self.columns {
                let testing = self[&Coordinate {
                    row: row as i32,
                    column: column as i32,
                }]
                    .as_ref();

                if testing.is_some() && testing != searching_tile {
                    return testing.map(|x| &x.window);
                }
            }
        }

        None
    }

    pub fn get_leader<'a>(&'a self, tile: &'a Tile) -> &'a Tile {
        match tile.tile_type {
            TileType::Leader { .. } => tile,
            TileType::Regular(coordinate) => self[&coordinate].as_ref().unwrap(),
        }
    }

    pub fn get_leader_mut<'a>(&'a mut self, tile: &'a mut Tile) -> &'a mut Tile {
        match tile.tile_type {
            TileType::Leader { .. } => tile,
            TileType::Regular(coordinate) => self[&coordinate].as_mut().unwrap(),
        }
    }

    pub fn get_unique_leaders(&self, adj: Vec<Coordinate>) -> Vec<&Tile> {
        adj.into_iter()
            .filter_map(|x| {
                let tile = self[&x].as_ref()?;

                println!("found tile: {tile:?}");

                let coordinate = match tile.tile_type {
                    TileType::Leader { coord, .. } => coord,
                    TileType::Regular(coord) => coord,
                };

                Some(coordinate)
            })
            .fold(Vec::new(), |mut acc, val| {
                if !acc.contains(&val) {
                    acc.push(val);
                }

                acc
            })
            .into_iter()
            .map(|x| {
                println!("unique leaders print {x:?}");
                self[&x].as_ref().unwrap()
            })
            .collect::<Vec<_>>()
    }

    // * visibility *
    pub fn is_visible(&self, coord: Coordinate) -> Result<(), Point<i32, Logical>> {
        let Some(tile) = self[&coord].as_ref() else {
            return Ok(());
        };

        let Tile {
            tile_type: TileType::Leader { rows, cols, coord },
            ..
        } = self.get_leader(tile)
        else {
            unreachable!()
        };

        let rect = {
            let mut rect = self.viewport;
            rect.loc += self.viewport_offset;
            rect
        };

        let Point {
            x: left, y: top, ..
        } = self.get_position_raw(*coord);

        let bottom = top + self.cell_height * (2 * (*rows as i32) + 1);
        let right = left + self.cell_width * (2 * (*cols as i32) + 1);

        let vp_left = rect.loc.x;
        let vp_top = rect.loc.y;
        let vp_right = vp_left + rect.size.w;
        let vp_bottom = vp_top + rect.size.h;

        let delta_x = if left < vp_left {
            vp_left - left
        } else if right > vp_right {
            vp_right - right
        } else {
            0
        };

        let delta_y = if top < vp_top {
            vp_top - top
        } else if bottom > vp_bottom {
            vp_bottom - bottom
        } else {
            0
        };

        if delta_x == 0 && delta_y == 0 {
            Ok(())
        } else {
            Err(Point::new(delta_x, delta_y))
        }

        /*
        match (rect.contains(top_left), rect.contains(top_right), rect.contains(bottom_left), rect.contains(bottom_right)) {
            (true, true, true, true) => {},
            (true, true, true, false) => unreachable!(),
            (true, true, false, true) => unreachable!(),
            (true, true, false, false) => {
                // bottom side of window is outside on the bottom part of the rect
                let bottom_rect = rect.loc.y + rect.size.h;

                y_movement += bottom_rect - bottom_left.y;
            },
            (true, false, true, true) => unreachable!(),
            (true, false, true, false) => {
                // right side of window is outside in the right side of the rect
                let right_rect = rect.loc.x + rect.size.w;

                x_movement += right_rect - top_right.x;
            },
            (true, false, false, true) => unreachable!(), // i think?
            (true, false, false, false) => {
                // top left corner is the only one in the rect, all other verts are outside the bottom right
                let bottom_right_rect = rect.loc + rect.size.to_point();

                let diff = bottom_right_rect - bottom_right;

                x_movement += diff.x;
                y_movement += diff.y;
            },
            (false, true, true, true) => unreachable!(),
            (false, true, true, false) => unreachable!(),
            (false, true, false, true) => {
                // left part of window is outside on the left part of the rect
                let left_rect = rect.loc.x;

                x_movement += left_rect - top_left.x;
            },
            (false, true, false, false) => {
                // top right corner is the only one in the rect, all other verts are outside the bottom left
                let bottom_left_rect = rect.loc + Point::new(0, rect.size.h);

                let diff = bottom_left_rect - bottom_left;

                x_movement += diff.x;
                y_movement += diff.y;
            },
            (false, false, true, true) => {
                // top side of window is outside on the top part of the rect
                let top_rect = rect.loc.y;

                y_movement += top_rect - top_left.y;
            },
            (false, false, true, false) => {
                // bottom left corner is the only one inside, the other three are outside in the top right
                let top_right_rect = rect.loc + Point::new(rect.size.w, 0);

                let diff = top_right_rect - top_right;

                x_movement += diff.x;
                y_movement += diff.y;
            },
            (false, false, false, true) => {
                // bottom right corner is the only one inside, the other three are outside in the top left
                let top_left_rect = rect.loc;

                let diff = top_left_rect - top_left;

                x_movement += diff.x;
                y_movement += diff.y;
            },
            (false, false, false, false) => {
                // no corners are inside

                // get positioning:
                //       \                \
                //   LT  \       T        \  RT
                // - - - - - - - - - - - - - - - -
                //       \||||||||||||||||\
                //   L   \||||viewport||||\   R
                //       \||||||||||||||||\
                //       \||||||||||||||||\
                // - - - - - - - - - - - - - - - -
                //   LB  \       B        \  RB
                //       \                \


                let left_rect = rect.loc.x;
                let right_rect = left_rect + rect.size.w;
                let top_rect = rect.loc.y;
                let bottom_rect = top_rect + rect.size.h;

                let left: bool = top_left.x < left_rect;
                let top: bool  = top_left.y < top_rect;

                match (left, top) {
                    // LT
                    (true, true) => {
                        x_movement += left_rect - top_left.x;
                        y_movement += top_rect - top_left.y;
                    },
                    // L or LB
                    (true, false) => {
                        x_movement += left_rect - bottom_left.x;

                        if bottom_rect < top_left.y || bottom_rect < bottom_left.y  {
                            // LB
                            y_movement += bottom_rect - bottom_left.y;
                        } // else it's L and it doesnt't need to move in y
                    },
                    // T or RT
                    (false, true) => {
                        y_movement += top_rect - top_right.y;

                        if right_rect < top_right.x || right_rect < top_left.x {
                            // RT
                            x_movement += right_rect - top_right.x;
                        } // else it's T
                    },
                    // B, R, or BR
                    (false, false) => {
                        // TODO ...
                    },
                }
            },
        }

        if x_movement == 0 && y_movement == 0 {
            Ok(())
        } else {
            Err(Point::new(x_movement, y_movement))
        }
        */
    }

    pub fn shift_all(&mut self, delta: Point<i32, Logical>, space: &Space<WayfleetWindow>) {
        self.viewport_offset -= delta;

        let mut anim = self.animation.write().unwrap();
        for row in &self.map {
            for tile in row {
                if let Some(tile) = tile.as_ref() {
                    anim.schedule::<MoveAnimation>(
                        InfoType::Delta(delta),
                        tile.window.clone(),
                        space,
                        Duration::from_millis(150),
                        Easing::EaseInOut,
                    );
                }
            }
        }
    }
}
