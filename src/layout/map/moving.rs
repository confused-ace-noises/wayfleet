use std::{mem, time::Duration};

use smithay::desktop::{Space, Window};

use crate::animations::{Easing, InfoType, MoveAnimation};

use super::{Map, coordinate::{Coordinate, Direction}, tile::{Tile, TileType}};

pub enum DoMove {
    Move(Vec<MoveInstructions>),

    NoMove,
}

pub struct MoveInstructions {
    pub old: Coordinate,
    pub new: Coordinate,
}



impl Map {
    pub fn is_there_space_and_move(
        &mut self,
        coord: &Coordinate,
        space: &mut Space<Window>,
        direction: Direction,
    ) -> Option<bool> {
        let x @ Tile {
            tile_type: TileType::Leader { coord, .. },
            ..
        } = self.get_leader(self[coord].as_ref()?).clone()
        else {
            println!("{:?}", self.map);
            unreachable!()
        };

        let adj = x.find_adjacent(self, &direction);
        let new_leader_coord = coord.step_towards(direction);
        let mut moves = Vec::new();

        for coord in adj {
            let maybe_tile_way = self
                .map
                .get(coord.row as usize)
                .map(|x| x.get(coord.column as usize).map(|x| x.as_ref()));

            if let Some(Some(tile_way)) = maybe_tile_way {
                // we didn't hit a boundry
                if tile_way.is_some() {
                    // check space
                    let do_move = self.is_there_space(&coord, direction).unwrap();

                    match do_move {
                        DoMove::Move(mut items) => moves.append(&mut items),
                        DoMove::NoMove => return Some(false),
                    }
                }
            } else {
                // hit a boundry
                return Some(false);
            }
        }

        // if it didn't return yet, all the checks passed: move
        // SAFETY: we checked if there were tiles in the way, and there should be none
        for MoveInstructions { old, new } in moves.iter() {
            // SAFETY: is_there_space checked.
            unsafe { self.move_tile(old, new, space) };
        }

        unsafe {
            self.move_tile(&coord, &new_leader_coord, space);
        }

        self.recalculate_available();

        Some(true)
    }

    pub fn is_there_space(&mut self, coord: &Coordinate, direction: Direction) -> Option<DoMove> {
        let x @ Tile {
            tile_type: TileType::Leader { coord, .. },
            ..
        } = self.get_leader(self[coord].as_ref()?).clone()
        else {
            unreachable!()
        };

        let adj = x.find_adjacent(self, &direction);
        let new_leader_coord = coord.step_towards(direction);

        let mut moves = Vec::with_capacity(adj.len());

        for coord in adj {
            let maybe_tile_way = self
                .map
                .get(coord.row as usize)
                .map(|x| x.get(coord.column as usize).map(|x| x.as_ref()));

            if let Some(Some(tile_way)) = maybe_tile_way {
                // we didn't hit a boundry
                if tile_way.is_some() {
                    // there's a tile in the way, recurse
                    let do_move = self.is_there_space(&coord, direction).unwrap();

                    match do_move {
                        DoMove::Move(mut items) => moves.append(&mut items),
                        DoMove::NoMove => return Some(DoMove::NoMove),
                    }
                }
            } else {
                // hit a boundry
                return Some(DoMove::NoMove);
            }
        }

        moves.push(MoveInstructions {
            old: coord,
            new: new_leader_coord,
        });
        Some(DoMove::Move(moves))
    }

    /// # Safety
    /// doesn't check if there's already a window where you wanna move
    pub unsafe fn move_tile(
        &mut self,
        old_coord: &Coordinate,
        new_leader_coord: &Coordinate,
        space: &mut Space<Window>,
    ) {
        unsafe { self.move_tile_replace(old_coord, new_leader_coord, None, space) };
    }

    /// # Safety
    /// doesn't check if there's already a window where you wanna move
    pub unsafe fn move_tile_replace(
        &mut self,
        old_coord: &Coordinate,
        new_leader_coord: &Coordinate,
        replace: Option<Tile>,
        space: &mut Space<Window>,
    ) {
        let Tile {
            tile_type: TileType::Leader { rows, cols, coord },
            window,
        } = self.get_leader(self[old_coord].as_ref().unwrap())
        else {
            unreachable!()
        };

        if *coord == *new_leader_coord {
            return;
        }

        let rows = *rows;
        let cols = *cols;
        let window = window.clone();

        let leader_pos = *coord;

        let mut tmp = Vec::with_capacity((rows + 1) * (cols + 1));

        for row in 0..=(rows as i32) {
            for col in 0..=(cols as i32) {
                let old = mem::replace(
                    &mut self[&Coordinate {
                        row: leader_pos.row + row,
                        column: leader_pos.column + col,
                    }],
                    replace.clone(),
                );

                tmp.push(old);
            }
        }

        // reverse order so pop() works
        for row in (rows as i32)..=0 {
            for col in (cols as i32)..=0 {
                let coord = Coordinate {
                    row: new_leader_coord.row + row,
                    column: new_leader_coord.column + col,
                };

                let mut popped = tmp.pop().unwrap();

                if let Some(Tile {
                    ref mut tile_type, ..
                }) = popped
                {
                    match tile_type {
                        TileType::Leader { coord, .. } => *coord = *new_leader_coord,
                        TileType::Regular(coordinate) => *coordinate = *new_leader_coord,
                    }
                }

                self[&coord] = popped;
            }
        }

        let mut anim_lock = self.animation.write().unwrap();

        anim_lock.schedule::<MoveAnimation>(
            InfoType::Final(self.get_position(*new_leader_coord)),
            window,
            space,
            Duration::from_millis(150),
            Easing::EaseInOut,
        );
    }
}