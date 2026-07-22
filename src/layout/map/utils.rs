use smithay::{
    desktop::{Space, Window},
    utils::{Logical, Point, Rectangle, Size},
};

use crate::layout::map::coordinate::Direction;

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
    pub fn find_window(&self, point: Point<i32, Logical>) -> Option<&Window> {
        for row in 0..self.rows {
            let row_rect = Rectangle::new(
                self.get_position(Coordinate {
                    row: row as i32,
                    column: 0,
                }),
                Size::new(self.total_width(), self.cell_height),
            );

            if row_rect.contains(point) {
                let row = &self.map[row];
                // found row, now find column
                for col in row.iter() {
                    if let Some(window) = col.as_ref()
                        && window.geometry().contains(point)
                    {
                        return Some(window);
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
        space: &Space<Window>,
    ) -> Option<(&Window, Point<i32, Logical>)> {
        for row in 0..self.rows {
            let row_rect = Rectangle::new(
                self.get_position(Coordinate {
                    row: row as i32,
                    column: 0,
                }),
                Size::new(self.total_width(), self.cell_height),
            );

            if row_rect.contains(point) {
                let row = &self.map[row];
                // found row, now find column
                for col in row.iter() {
                    if let Some(window) = col
                        && window.geometry().contains(point)
                    {
                        let pos = space.element_location(window).unwrap();
                        return Some((window, pos));
                    }
                }

                // didn't find it here, the pointer is in some random gap
                break;
            }
        }

        None
    }

    pub fn get_position(&self, Coordinate { row, column }: Coordinate) -> Point<i32, Logical> {
        Point::new(
            column * self.cell_width + (column) * self.spaces.horizontal as i32 + self.offset.x,
            row * self.cell_height + (row) * self.spaces.vertical as i32 + self.offset.y,
        ) + Point::new(column.signum(), row.signum())
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

    pub fn search_tile(&self, searched_window: &Window) -> Option<Coordinate> {
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
    ) -> Option<&Window> {
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

    pub fn radial_search(&self, search_from: Coordinate) -> Option<&Window> {
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
}
