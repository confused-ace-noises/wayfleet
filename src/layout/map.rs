use std::{
    collections::HashSet,
    mem,
    ops::{Deref, DerefMut, Index, IndexMut},
};

use smithay::{
    desktop::{Space, Window},
    reexports::rustix::net::ipproto::TP,
    utils::{Logical, Point, Rectangle, Size},
};

use crate::layout::{
    controller::{LayoutController, ResizeType},
    map::TileType::Regular,
};

#[derive(Debug, Clone)]
pub struct Tile {
    pub window: Window,
    pub tile_type: TileType,
}

#[derive(Debug, Clone)]
pub enum TileType {
    Leader { rows: usize, cols: usize },
    Regular(Coordinate),
}

impl Tile {
    pub fn new(window: Window) -> Self {
        Self {
            window,
            tile_type: TileType::Leader { rows: 0, cols: 0 },
        }
    }

    pub fn new_siblings(window: Window, tile_type: TileType) -> Self {
        Self { window, tile_type }
    }
}

impl Deref for Tile {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DerefMut for Tile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

// TODO: add multi-cell windows, probably
pub struct Map {
    pub map: Vec<Vec<Option<Tile>>>,
    pub first_available: Option<Coordinate>,
    pub rows: usize,
    pub columns: usize,
    pub cell_height: i32,
    pub cell_width: i32,
    pub offset: Point<i32, Logical>,
}

impl Map {
    pub fn new(
        rows: usize,
        columns: usize,
        cell_height: i32,
        cell_width: i32,
        offset: Point<i32, Logical>,
    ) -> Self {
        assert_ne!(rows, 0);
        assert_ne!(columns, 0);
        assert!(cell_height > 0);
        assert!(cell_width > 0);

        Self {
            map: vec![vec![None; columns]; rows],
            first_available: Some([0, 0].into()),
            rows,
            columns,
            cell_height,
            cell_width,
            offset,
        }
    }

    pub fn insert(&mut self, window: Window) -> Option<Coordinate> {
        if let Some(coord) = self.first_available {
            self[&coord] = Some(Tile::new(window));
            self.recalculate_available();
            Some(coord)
        } else {
            None
        }
    }

    pub fn insert_at(&mut self, window: Window, position: &Coordinate) -> bool {
        let x = &mut self[position];

        if x.is_none() {
            *x = Some(Tile::new(window));
            if let Some(available) = self.first_available
                && *position == available
            {
                self.recalculate_available();
            }
            true
        } else {
            false
        }
    }

    pub fn change_cells(
        &mut self,
        position: &Coordinate,
        direction: Direction,
        space: &mut Space<Window>,
    ) -> Option<bool> {
        let current = self[position].as_ref()?;
        let (Tile { window, tile_type: TileType::Leader { rows, cols } }, coord_leader) = self.get_leader(current) else { unreachable!() };

        let mut rows = *rows;
        let mut cols = *cols;
        let window = window.clone();

        let coord_leader = coord_leader.unwrap_or(*position);

        match direction {
            Direction::Right => {
                let coords = (coord_leader.row..=(coord_leader.row + rows as i32))
                    .map(|row| Coordinate {
                        row,
                        column: coord_leader.column + cols as i32 + 1,
                    })
                    .collect::<Vec<_>>();

                for coord in coords {
                    if let Some(false) = self.is_there_space_and_move(&coord, space, direction) {
                        return Some(false)
                    }
                }

                LayoutController::resize_delta(&window, ResizeType::Width(self.cell_width));

                let Tile { tile_type: TileType::Leader { rows, cols }, .. } = self[&coord_leader].as_mut().unwrap() else { unreachable!() };
                *cols += 1;
            },
            Direction::Down => {
                
                
                rows += 1;
            },
            Direction::Up => {
                // rows += 1;
                // leader_tile.tile_type = TileType::Regular(Coordinate {
                //     row: leader_coord.row - 1,
                //     column: leader_coord.column,
                // })
            }
            Direction::Left => todo!(),
            Direction::RmUp => todo!(),
            Direction::RmDown => todo!(),
            Direction::RmLeft => todo!(),
            Direction::RmRight => todo!(),
        }
        /*
        match direction {
            Direction::Up | Direction::Down => {
                let movement = {
                    if let Direction::Up = direction {
                        -1
                    } else {
                        1
                    }
                };

                resize.h += self.cell_height;
                let mut pos = *position;
                // find min row
                loop {
                    pos.row += movement;
                    if !set.contains(&pos) {
                        pos.row -= movement;
                        break;
                    }
                }

                if pos.row <= 0 || pos.row >= self.rows as i32 { // TODO: check if this is right
                    return Some(());
                }

                let top_column_start_point = pos.column;
                // find min column
                loop {
                    pos.column -= 1;
                    if !set.contains(&pos) {
                        if let Direction::Up = direction {
                            new_window_pos = Coordinate {
                                row: pos.row,
                                column: pos.column + 1,
                            };
                        } else {
                            new_window_pos = Coordinate {
                                row: pos.row,
                                column: pos.column,
                            };
                        }
                        pos.column = top_column_start_point;
                        break;
                    }
                    set.insert(Coordinate {
                        row: pos.row + movement,
                        column: pos.column,
                    });
                }

                loop {
                    pos.column += 1;
                    if !set.contains(&pos) {
                        // pos.column = top_column_start_point;
                        break;
                    }
                    set.insert(Coordinate {
                        row: pos.row + movement,
                        column: pos.column,
                    });
                }
            },
            Direction::Left | Direction::Right => {

            },
            Direction::RmUp => todo!(),
            Direction::RmDown => todo!(),
            Direction::RmLeft => todo!(),
            Direction::RmRight => todo!(),
        }

        LayoutController::resize(window, ResizeType::Both(resize));
        space.relocate_element(
            window,
            Point::new(
                new_window_pos.column * self.cell_width + self.offset.x,
                new_window_pos.row * self.cell_height + self.offset.y,
            ),
        );
        */
        Some(true)
    }

    pub fn is_there_space_and_move(
        &mut self,
        coord: &Coordinate,
        space: &mut Space<Window>,
        direction: Direction,
    ) -> Option<bool> {
        let (
            Tile {
                tile_type: TileType::Leader { rows, cols },
                ..
            },
            coord_leader,
        ) = self.get_leader(self[coord].as_ref()?)
        else {
            unreachable!()
        };

        let coord_leader = coord_leader.unwrap_or(*coord);

        let rows = *rows;
        let cols = *cols;

        let coords;
        let new_leader_coord;
        match direction {
            Direction::Right => {
                coords = (coord_leader.row..=(coord_leader.row + rows as i32))
                    .map(|row| Coordinate {
                        row,
                        column: coord_leader.column + cols as i32 + 1,
                    })
                    .collect::<Vec<_>>();

                new_leader_coord = Coordinate {
                    row: coord_leader.row,
                    column: coord_leader.column + 1,
                };
            }
            Direction::Up => {
                coords = (coord_leader.column..=(coord_leader.column + cols as i32))
                    .map(|column| Coordinate {
                        row: coord_leader.row - 1,
                        column,
                    })
                    .collect::<Vec<_>>();
                new_leader_coord = Coordinate {
                    row: coord_leader.row - 1,
                    column: coord_leader.column,
                };
            }
            Direction::Down => {
                coords = (coord_leader.column..=(coord_leader.column + cols as i32))
                    .map(|column| Coordinate {
                        row: coord_leader.row + rows as i32 + 1,
                        column,
                    })
                    .collect::<Vec<_>>();
                new_leader_coord = Coordinate {
                    row: coord_leader.row + 1,
                    column: coord_leader.column,
                };
            }
            Direction::Left => {
                coords = (coord_leader.row..=(coord_leader.row + rows as i32))
                    .map(|row| Coordinate {
                        row,
                        column: coord_leader.column - 1,
                    })
                    .collect::<Vec<_>>();
                new_leader_coord = Coordinate {
                    row: coord_leader.row,
                    column: coord_leader.column - 1,
                };
            }
            _ => return None,
        }

        // returns here before panicking????
        dbg!(&coords);
        dbg!(&new_leader_coord);

        for coord in coords {
            let maybe_tile_way = self
                .map
                .get(coord.row as usize)
                .map(|x| x.get(coord.column as usize).map(|x| x.as_ref()));

            dbg!(coord);

            dbg!(&maybe_tile_way);

            if let Some(Some(tile_way)) = maybe_tile_way {
                println!("im here");
                // we didn't hit a boundry
                if tile_way.is_some() {
                    // there's a tile in the way, recurse
                    if !self
                        .is_there_space_and_move(&coord, space, direction)
                        .unwrap()
                    {
                        return Some(false)
                    }
                }
            } else {
                // hit a boundry
                println!("hit boundry");
                return Some(false)
            }
        }

        // if it didn't return yet, all the checks passed: move
        // SAFETY: we checked if there were tiles in the way, and there should be none
        unsafe { self.move_tile(&coord_leader, &new_leader_coord, space) };
        Some(true)
    }

    pub fn get_leader<'a>(&'a self, tile: &'a Tile) -> (&'a Tile, Option<Coordinate>) {
        match tile.tile_type {
            TileType::Leader { .. } => (tile, None),
            Regular(coordinate) => (self[&coordinate].as_ref().unwrap(), Some(coordinate)),
        }
    }

    /// # Safety
    /// doesn't check if there's already a window where you wanna move
    pub unsafe fn move_tile(
        &mut self,
        old_coord: &Coordinate,
        new_leader_coord: &Coordinate,
        space: &mut Space<Window>,
    ) {
        let (
            Tile {
                tile_type: TileType::Leader { rows, cols },
                window,
            },
            pos,
        ) = self.get_leader(self[old_coord].as_ref().unwrap())
        else {
            unreachable!()
        };

        let rows = *rows;
        let cols = *cols;
        let window = window.clone();

        let leader_pos = pos.unwrap_or(*old_coord);

        let mut tmp = Vec::with_capacity((rows + 1) * (cols + 1));

        for row in 0..=(rows as i32) {
            for col in 0..=(cols as i32) {
                let old = mem::take(
                    &mut self[&Coordinate {
                        row: leader_pos.row + row,
                        column: leader_pos.column + col,
                    }],
                );

                tmp.push(old);
            }
        }

        // reverse order so pop() works
        for row in (rows as i32)..=0 {
            for col in (cols as i32)..=0 {
                self[&Coordinate {
                    row: new_leader_coord.row + row,
                    column: new_leader_coord.column + col,
                }] = tmp.pop().unwrap();
            }
        }
        space.relocate_element(&window, self.get_position(*new_leader_coord));
    }

    // pub fn remove(&mut self, position: &Coordinate) -> Option<Vec<Tile>> {
    //     let mut res = Vec::new();
    //     let first = mem::take(&mut self[position])?;

    //     let siblings = first.siblings.iter().filter_map(|x| self.remove_single(x));
    //     res.extend(siblings);
    //     res.push(first);
    //     let push = res.swap_remove(0);
    //     res.push(push);

    //     Some(res)
    // }

    fn remove_single(&mut self, position: &Coordinate) -> Option<Tile> {
        mem::take(&mut self[position])
    }

    pub fn recalculate_available(&mut self) {
        if let Some(Coordinate { row, mut column }) = self.first_available {
            let mut found = false;
            // first try, in front
            'outer: for r in (row as usize)..self.rows {
                for c in (column as usize)..self.columns {
                    if self.map[r][c].is_none() {
                        self.first_available = Some(Coordinate {
                            row: r as i32,
                            column: c as i32,
                        });
                        found = true;
                        break 'outer;
                    }
                }
                column = 0;
            }

            // try behind
            if !found {
                'outer: for r in 0..=row {
                    for c in 0..=column {
                        if self.map[r as usize][c as usize].is_none() {
                            self.first_available = Some(Coordinate { row: r, column: c });
                            found = true;
                            break 'outer;
                        }
                    }
                }
            }

            // still hasn't been found, all places are full
            if !found {
                self.first_available = None
            }
        } else {
            'outer: for r in 0..self.rows {
                for c in 0..self.columns {
                    if self.map[r][c].is_none() {
                        self.first_available = Some(Coordinate {
                            row: r as i32,
                            column: c as i32,
                        });
                        break 'outer;
                    }
                }
            }
        }
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
            column * self.cell_width + self.offset.x,
            row * self.cell_height + self.offset.y,
        )
    }

    pub fn get_size(&self) -> Size<i32, Logical> {
        Size::new(self.cell_width, self.cell_width)
    }

    pub fn total_width(&self) -> i32 {
        self.cell_width * self.columns as i32
    }
}

impl Index<&Coordinate> for Map {
    type Output = Option<Tile>;

    fn index(&self, Coordinate { row, column }: &Coordinate) -> &Self::Output {
        &self.map[*row as usize][*column as usize]
    }
}

impl IndexMut<&Coordinate> for Map {
    fn index_mut(&mut self, Coordinate { row, column }: &Coordinate) -> &mut Self::Output {
        &mut self.map[*row as usize][*column as usize]
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,

    RmUp,
    RmDown,
    RmLeft,
    RmRight,
}

impl Direction {

}

#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Hash)]
pub struct Coordinate {
    pub row: i32,
    pub column: i32,
}

impl From<(i32, i32)> for Coordinate {
    fn from(value: (i32, i32)) -> Self {
        Self {
            row: value.0,
            column: value.1,
        }
    }
}

impl From<[i32; 2]> for Coordinate {
    fn from(value: [i32; 2]) -> Self {
        Self {
            row: value[0],
            column: value[1],
        }
    }
}
