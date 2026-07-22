use std::ops::{Deref, DerefMut};

use smithay::desktop::Window;

use super::{Map, coordinate::{Coordinate, Direction}};

#[derive(Debug, Clone)]
pub struct Tile {
    pub window: Window,
    pub tile_type: TileType,
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        self.leader_coord() == other.leader_coord()
    }
}

#[derive(Debug, Clone)]
pub enum TileType {
    Leader {
        rows: usize,
        cols: usize,
        coord: Coordinate,
    },
    Regular(Coordinate),
}

impl Tile {
    pub fn new_leader(window: Window, coord: Coordinate) -> Self {
        Self {
            window,
            tile_type: TileType::Leader {
                rows: 0,
                cols: 0,
                coord,
            },
        }
    }

    pub fn new_regular(window: Window, coord: Coordinate) -> Self {
        Self {
            window,
            tile_type: TileType::Regular(coord),
        }
    }

    pub fn leader_coord(&self) -> Coordinate {
        match self.tile_type {
            TileType::Leader { coord, .. } => coord,
            TileType::Regular(coordinate) => coordinate,
        }
    }

    pub fn project<'a>(&self, map: &'a Map, delta: Coordinate) -> Vec<&'a Tile> {
        let root_leader = map.get_leader(self);

        let mut vec = self.bounding_coords(map).into_iter().filter_map(|x| {
            
            
            let coord = x + delta;

            println!("delta: {delta:?}");

            let tile = map.map.get(coord.row as usize)?.get(coord.column as usize)?.as_ref()?;

            let leader = map.get_leader(tile);
            
            if *root_leader == *leader {
                None
            } else {
                Some(leader)
            }
            
        }).collect::<Vec<_>>();

        vec.dedup();

        vec
    }

    pub fn bounding_coords(&self, map: &Map) -> Vec<Coordinate> {
        let TileType::Leader { rows, cols, coord } = map.get_leader(self).tile_type else { unreachable!() };

        let mut coords = Vec::with_capacity((rows+1)*(cols+1));

        for row in 0..=rows {
            for col in 0..=cols {
                coords.push(coord + Coordinate { row: row as i32, column: col as i32 });
            }
        }

        coords
    }

    pub fn find_adjacent(&self, map: &Map, direction: &Direction) -> Vec<Coordinate> {
        let Tile {
            tile_type: TileType::Leader { rows, cols, coord },
            ..
        } = map.get_leader(self)
        else {
            unreachable!()
        };

        match direction {
            Direction::Up => (coord.column..=(coord.column + *cols as i32))
                .map(|column| Coordinate {
                    row: coord.row - 1,
                    column,
                })
                .collect(),
            Direction::Down => (coord.column..=(coord.column + *cols as i32))
                .map(|column| Coordinate {
                    row: coord.row + *rows as i32 + 1,
                    column,
                })
                .collect(),
            Direction::Left => (coord.row..=(coord.row + *rows as i32))
                .map(|row| Coordinate {
                    row,
                    column: coord.column - 1,
                })
                .collect(),
            Direction::Right => (coord.row..=(coord.row + *rows as i32))
                .map(|row| Coordinate {
                    row,
                    column: coord.column + *cols as i32 + 1,
                })
                .collect(),
        }
    }

    pub fn find_outskirts(&self, map: &Map, direction: &Direction) -> Vec<Coordinate> {
        let Tile {
            tile_type: TileType::Leader { rows, cols, coord },
            ..
        } = map.get_leader(self)
        else {
            unreachable!()
        };

        match direction {
            Direction::Up => (coord.column..=(coord.column + *cols as i32))
                .map(|column| Coordinate {
                    row: coord.row,
                    column,
                })
                .collect(),
            Direction::Down => (coord.column..=(coord.column + *cols as i32))
                .map(|column| Coordinate {
                    row: coord.row + *rows as i32,
                    column,
                })
                .collect(),
            Direction::Left => (coord.row..=(coord.row + *rows as i32))
                .map(|row| Coordinate {
                    row,
                    column: coord.column,
                })
                .collect(),
            Direction::Right => (coord.row..=(coord.row + *rows as i32))
                .map(|row| Coordinate {
                    row,
                    column: coord.column + *cols as i32,
                })
                .collect(),
        }
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