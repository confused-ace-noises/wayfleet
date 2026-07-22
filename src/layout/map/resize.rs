use std::time::{Duration, Instant};

use smithay::{desktop::{Space, Window}, utils::{Logical, Size}};

use crate::animations::{AnimationBase, Easing, InfoType, MoveAnimation, ResizeAnimation};

use super::{Map, coordinate::{Coordinate, Direction}, tile::{Tile, TileType}, moving::{DoMove, MoveInstructions}};

impl Map {
    pub fn change_cells(
        &mut self,
        position: &Coordinate,
        direction: Direction,
        remove: bool,
        space: &mut Space<Window>,
    ) -> Option<bool> {
        let current = self[position].as_ref()?;
        let tile @ Tile {
            tile_type:
                TileType::Leader {
                    coord,
                    mut rows,
                    mut cols,
                },
            ..
        } = self.get_leader(current).clone()
        else {
            unreachable!()
        };
        let new_coord = coord.step_towards_expand(direction, remove);

        if remove {
            let outskirts = tile.find_outskirts(self, &direction);

            for coordinate in outskirts {
                self[&coordinate] = None;
            }

            let mut anim_lock = self.animation.write().unwrap();

            let mut anim_delta: Size<i32, Logical> = Size::new(0, 0);

            if let Direction::Up | Direction::Down = direction {
                if rows == 1 {
                    return None;
                }
                rows -= 1;
                anim_delta.h += -self.cell_height - self.spaces.vertical as i32;
            } else {
                if cols == 1 {
                    return None;
                }
                cols -= 1;
                anim_delta.w += -self.cell_width - self.spaces.horizontal as i32;
            }

            let start = Instant::now();

            if new_coord != coord {
                anim_lock.schedule_specific(
                    AnimationBase::<MoveAnimation>::new_with_time(
                        InfoType::Final(self.get_position(new_coord)),
                        tile.window.clone(),
                        space,
                        Duration::from_millis(150),
                        Easing::EaseInOut,
                        start,
                        // this is just a magic number, i thought it should
                        // be 1 but it works with 2 for some reason? idk
                        2,
                    ),
                );
            }
            // let start = Instant::now();
            anim_lock.schedule_specific(
                AnimationBase::<ResizeAnimation>::new_with_time(
                    InfoType::Delta(anim_delta),
                    tile.window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                    start,
                    0,
                ),
            );

            drop(anim_lock);

            self[&new_coord] = Some(Tile {
                window: tile.window,
                tile_type: TileType::Leader {
                    rows,
                    cols,
                    coord: new_coord,
                },
            });

            // SAFETY:
            // the window getting shrinked is guaranteed to be included in (rows, cols)
            unsafe { self.repoint_regualr_tiles(new_coord) };
            return Some(true);
        }

        let adj = tile.find_adjacent(self, &direction);

        let mut moves = Vec::new();

        for coordinate in adj.iter() {
            match self.is_there_space(coordinate, direction) {
                Some(DoMove::Move(items)) => moves.extend(items),
                Some(DoMove::NoMove) => return Some(false),
                None => continue,
            }
        }

        for MoveInstructions { old, new } in moves.iter() {
            // SAFETY: is_there_space checked.
            unsafe { self.move_tile(old, new, space) };
        }

        for coordinate in adj {
            self[&coordinate] = Some(Tile::new_regular(tile.window.clone(), coord))
        }

        let cell_width = self.cell_width;
        let cell_height = self.cell_height;

        let spaces = self.spaces;

        let Tile {
            tile_type: TileType::Leader { rows, cols, .. },
            ..
        } = self[&new_coord].as_mut().unwrap()
        else {
            unreachable!()
        };

        let anim_delta: Size<i32, Logical>;

        if let Direction::Down | Direction::Up = direction {
            anim_delta = Size::new(0, cell_height + spaces.vertical as i32);
            *rows += 1;
        } else {
            anim_delta = Size::new(cell_width + spaces.horizontal as i32, 0);
            *cols += 1;
        }

        {
            let mut anim_lock = self.animation.write().unwrap();

            anim_lock.schedule::<ResizeAnimation>(
                InfoType::Delta(anim_delta),
                tile.window,
                space,
                Duration::from_millis(150),
                Easing::EaseInOut
            );
        }

        unsafe {
            self.move_tile(&coord, &new_coord, space);
        }

        self.recalculate_available();

        Some(true)
    }
}