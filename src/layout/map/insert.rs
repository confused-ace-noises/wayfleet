use std::mem;

use smithay::desktop::{Space, Window};

use super::{Map, coordinate::Coordinate, tile::{Tile, TileType}};

impl Map {
    pub fn insert(&mut self, window: Window) -> Option<Coordinate> {
        if let Some(coord) = self.first_available {
            self[&coord] = Some(Tile::new_leader(window, coord));
            self.recalculate_available();
            Some(coord)
        } else {
            None
        }
    }

    pub fn insert_at(&mut self, window: Window, position: &Coordinate) -> bool {
        let x = &mut self[position];

        if x.is_none() {
            *x = Some(Tile::new_leader(window, *position));
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

    pub fn remove(&mut self, position: &Coordinate, space: &mut Space<Window>) -> Option<Vec<Tile>> {
        if let Some(x) = self.focus && x == *position {
            self.focus = None
        } 

        let tile = self[position].as_ref()?;
        let Tile { tile_type: TileType::Leader { rows, cols, coord }, ref window }: Tile = *self.get_leader(tile) else { unreachable!() };

        space.unmap_elem(window);

        let mut vec = vec![];

        for r in 0..=rows {
            for c in 0..=cols {
                vec.push(mem::take(&mut self.map[coord.row as usize + r][coord.column as usize + c]))
            }
        }

        self.recalculate_available();

        if vec.iter().any(Option::is_none) {
            None
        } else {
            Some(vec.into_iter().flatten().collect())   
        }
    }

    #[allow(unused)]
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
}