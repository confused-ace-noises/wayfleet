use std::{collections::HashSet, time::Duration};

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
            column * self.cell_width + (column) * self.spaces.horizontal as i32 + self.viewport.loc.x,
            row * self.cell_height + (row) * self.spaces.vertical as i32 + self.viewport.loc.y,
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
        self.search_tile_window(&searched_window.window)
    }
    
    pub fn search_tile_window(&self, searched: &Window) -> Option<Coordinate> {
        for r in 0..self.rows {
            for c in 0..self.columns {
                let coord = (r as i32, c as i32).into();
                if let Some(Tile { window, .. }) = &self[&coord]
                    && *window == *searched
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

        let bottom = top + self.cell_height * ((*rows as i32) + 1);
        let right = left + self.cell_width * ((*cols as i32) + 1);

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
    }

    pub fn shift_all(&mut self, delta: Point<i32, Logical>, space: &Space<WayfleetWindow>) {
        self.viewport_offset -= delta;

        let mut anim = self.animation.write().unwrap();

        let mut done_leaders = HashSet::new();

        for row in &self.map {
            for tile in row {
                if let Some(tile) = tile.as_ref() {
                    let leader = tile.leader_coord();

                    if !done_leaders.contains(&leader) {
                        done_leaders.insert(leader);
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
}
