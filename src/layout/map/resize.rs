use std::{
    collections::HashSet, time::Duration,
};

use smithay::{
    desktop::Space, utils::Size,
};

use crate::{
    animations::{Easing, InfoType, MoveAnimation, ResizeAnimation}, layout::{WayfleetWindow, controller::LayoutController},
};

use super::{
    Map,
    coordinate::{Coordinate, Direction},
    moving::{DoMove, MoveInstructions},
    tile::{Tile, TileType},
};

impl Map {
    pub fn change_cells_focused(
        &mut self,
        direction: Direction,
        remove: bool,
        space: &mut Space<WayfleetWindow>
    ) -> Option<bool> {
        let focus = self.focus?;
        self.change_cells(&focus, direction, remove, space) 
    }

    pub fn change_cells(
        &mut self,
        position: &Coordinate,
        direction: Direction,
        remove: bool,
        space: &mut Space<WayfleetWindow>,
    ) -> Option<bool> {
        if !remove {
            self.resize_single_window_add(position, direction, space)
        } else {
            self.resize_single_window_remove(position, direction, space)
        }
    }

    // TODO: remove is broken
    // pub fn change_cells(
    //     &mut self,
    //     position: &Coordinate,
    //     direction: Direction,
    //     remove: bool,
    //     space: &mut Space<WayfleetWindow>,
    // ) -> Option<bool> {
    //     let current = self[position].as_ref()?;
    //     let tile @ Tile {
    //         tile_type:
    //             TileType::Leader {
    //                 coord,
    //                 mut rows,
    //                 mut cols,
    //             },
    //         ..
    //     } = self.get_leader(current).clone()
    //     else {
    //         unreachable!()
    //     };

    //     let new_coord = coord.step_towards_expand(direction, remove);

    //     if remove {
    //         let outskirts = tile.find_outskirts(self, &direction);

    //         for coordinate in outskirts {
    //             self[&coordinate] = None;
    //         }

    //         let mut anim_lock = self.animation.write().unwrap();

    //         let mut anim_delta: Size<i32, Logical> = Size::new(0, 0);

    //         if let Direction::Up | Direction::Down = direction {
    //             if rows == 1 {
    //                 return None;
    //             }
    //             rows -= 1;
    //             anim_delta.h += -self.cell_height - self.spaces.vertical as i32;
    //         } else {
    //             if cols == 1 {
    //                 return None;
    //             }
    //             cols -= 1;
    //             anim_delta.w += -self.cell_width - self.spaces.horizontal as i32;
    //         }

    //         let start = Instant::now();

    //         if new_coord != coord {
    //             anim_lock.schedule_specific(AnimationBase::<MoveAnimation>::new_with_time(
    //                 InfoType::Final(self.get_position_shifted(new_coord)),
    //                 tile.window.clone(),
    //                 space,
    //                 Duration::from_millis(150),
    //                 Easing::EaseInOut,
    //                 start,
    //                 // this is just a magic number, i thought it should
    //                 // be 1 but it works with 2 for some reason? idk
    //                 2,
    //             ));
    //         }
    //         // let start = Instant::now();
    //         anim_lock.schedule_specific(AnimationBase::<ResizeAnimation>::new_with_time(
    //             InfoType::Delta(anim_delta),
    //             tile.window.clone(),
    //             space,
    //             Duration::from_millis(150),
    //             Easing::EaseInOut,
    //             start,
    //             0,
    //         ));

    //         drop(anim_lock);

    //         self[&new_coord] = Some(Tile {
    //             window: tile.window,
    //             tile_type: TileType::Leader {
    //                 rows,
    //                 cols,
    //                 coord: new_coord,
    //             },
    //         });

    //         // SAFETY:
    //         // the window getting shrinked is guaranteed to be included in (rows, cols)
    //         unsafe { self.repoint_regualr_tiles(new_coord) };
    //         return Some(true);
    //     }

    //     let adj = tile.find_adjacent(self, &direction);

    //     if adj.iter().any(|x| !self.is_valid_coord(*x)) {
    //         return Some(false);
    //     }

    //     let mut moves = Vec::new();

    //     for coordinate in adj.iter() {
    //         match self.is_there_space(coordinate, direction) {
    //             Some(DoMove::Move(items)) => moves.extend(items),
    //             Some(DoMove::NoMove) => return Some(false),
    //             None => continue,
    //         }
    //     }

    //     for MoveInstructions { old, new } in moves.iter() {
    //         // SAFETY: is_there_space checked.
    //         unsafe { self.move_tile(old, new, space) };
    //     }

    //     for coordinate in adj {
    //         self[&coordinate] = Some(Tile::new_regular(tile.window.clone(), coord))
    //     }

    //     let cell_width = self.cell_width;
    //     let cell_height = self.cell_height;

    //     let spaces = self.spaces;

    //     self[&new_coord] = Some(tile.clone());

    //     let Tile {
    //         tile_type: TileType::Leader { rows, cols, .. },
    //         ..
    //     } = self[&new_coord].as_mut().unwrap()
    //     else {
    //         unreachable!()
    //     };

    //     let anim_delta: Size<i32, Logical>;

    //     if let Direction::Down | Direction::Up = direction {
    //         anim_delta = Size::new(0, cell_height + spaces.vertical as i32);
    //         *rows += 1;
    //     } else {
    //         anim_delta = Size::new(cell_width + spaces.horizontal as i32, 0);
    //         *cols += 1;
    //     }

    //     {
    //         let mut anim_lock = self.animation.write().unwrap();

    //         anim_lock.schedule::<ResizeAnimation>(
    //             InfoType::Delta(anim_delta),
    //             tile.window.clone(),
    //             space,
    //             Duration::from_millis(150),
    //             Easing::EaseInOut,
    //         );

    //         anim_lock.schedule::<MoveAnimation>(
    //             InfoType::Final(self.get_position_shifted(new_coord)), 
    //             tile.window, 
    //             space, 
    //             Duration::from_millis(150),
    //             Easing::EaseInOut,
    //         );
    //     }

    //     // unsafe {
    //     //     self.move_tile(&coord, &new_coord, space);
    //     // }

    //     unsafe { self.repoint_regualr_tiles(new_coord) }; 

    //     self.recalculate_available();

    //     Some(true)
    // }

    pub fn resize_single_window_add(
        &mut self,
        position: &Coordinate,
        direction: Direction,
        space: &mut Space<WayfleetWindow>,
    ) -> Option<bool> {
        let called_on = self[position].as_ref()?;
        let old_leader @ Tile {
            tile_type:
                TileType::Leader {
                    coord: old_leader_coord,
                    ..
                },
            ..
        } = self.get_leader(called_on).clone()
        else {
            unreachable!()
        };

        let new_leader_coord = old_leader_coord.step_towards_expand(direction, false);

        let new_cells = old_leader.find_adjacent(self, &direction);

        if new_cells.iter().any(|cell| !self.is_valid_coord(*cell)) {
            return Some(false);
        }

        let mut moves = Vec::new();

        for new_cell in new_cells {
            match self.is_there_space(&new_cell, direction) {
                Some(DoMove::Move(items)) => moves.extend(items),
                Some(DoMove::NoMove) => return Some(false),
                None => continue,
            }
        }

        for MoveInstructions { old, new } in moves {
            unsafe {
                self.move_tile(&old, &new, space)
            }
        }

        // have cleared space to operate safely, now do so

        // 1. move leader to destination
        unsafe {
            self.move_tile(&old_leader_coord, &new_leader_coord, space);
        }

        // 2. update rows and cols of leader
        let cell_height = self.cell_height;
        let cell_width = self.cell_width;
        let spaces = self.spaces;

        let Tile { tile_type: TileType::Leader { rows, cols, .. }, window } = self[&new_leader_coord].as_mut().unwrap() else { unreachable!() };

        let window = window.clone();

        let mut delta_anim_size = Size::new(0, 0);

        if let Direction::Down | Direction::Up = direction {
            delta_anim_size.h += cell_height + spaces.vertical as i32;
            *rows += 1;
        } else {
            delta_anim_size.w += cell_width + spaces.horizontal as i32;
            *cols += 1;
        }

        // 3. repoint all stuff belonging to this leader
        unsafe { self.repoint_regualr_tiles(new_leader_coord) };

        // 4. animate
        let mut anim_lock = self.animation.write().unwrap();

        anim_lock.schedule::<ResizeAnimation>(
            InfoType::Delta(delta_anim_size), 
            window.clone(), 
            space, 
            Duration::from_millis(150), 
            Easing::EaseInOut
        );

        if old_leader_coord != new_leader_coord {
            anim_lock.schedule::<MoveAnimation>(
                InfoType::Final(self.get_position_shifted(new_leader_coord)), 
                window.clone(), 
                space, 
                Duration::from_millis(150), 
                Easing::EaseInOut
            );
        }

        drop(anim_lock);

        self.realign_focused(space); 
        self.recalculate_available();

        Some(true)
    }

    pub fn resize_single_window_remove(
        &mut self,
        position: &Coordinate,
        direction: Direction,
        space: &mut Space<WayfleetWindow>,
    ) -> Option<bool> {
        let called_on = self[position].as_ref()?;
        let old_leader @ Tile {
            tile_type:
                TileType::Leader {
                    coord: old_leader_coord,
                    rows: mut old_rows,
                    cols: mut old_cols
                },
            ..
        } = self.get_leader(called_on).clone()
        else {
            unreachable!()
        };

        let new_leader_coord = old_leader_coord.step_towards_expand(direction, true);
        
        // 1. remove old cells 
        let old_cells = old_leader.find_adjacent(self, &direction);

        for old_cell in old_cells {
            if self.is_valid_coord(old_cell) {
                self[&old_cell] = None;
            }
        }

        // 2. fix old dimensions
        let mut delta_anim_size = Size::new(0, 0);

        if let Direction::Down | Direction::Up = direction {
            delta_anim_size.h -= self.cell_height + self.spaces.vertical as i32;
            old_rows -= 1;
        } else {
            delta_anim_size.w -= self.cell_width + self.spaces.horizontal as i32;
            old_cols -= 1;
        }

        // 3. extablish new leader
        self[&new_leader_coord] = Some(Tile {
            window: old_leader.window.clone(),
            tile_type: TileType::Leader { rows: old_rows, cols: old_cols, coord: new_leader_coord },
        });

        // 4. repoint regular cells to leader
        unsafe { self.repoint_regualr_tiles(new_leader_coord) };

        // 5. do animations
        let mut anim_lock = self.animation.write().unwrap();

        anim_lock.schedule::<ResizeAnimation>(
            InfoType::Delta(delta_anim_size), 
            old_leader.window.clone(), 
            space, 
            Duration::from_millis(150), 
            Easing::EaseInOut
        );

        if old_leader_coord != new_leader_coord {
            anim_lock.schedule::<MoveAnimation>(
                InfoType::Final(self.get_position_shifted(new_leader_coord)), 
                old_leader.window.clone(), 
                space, 
                Duration::from_millis(150), 
                Easing::EaseInOut
            );
        }

        drop(anim_lock);

        self.realign_focused(space); 
        self.recalculate_available();

        Some(true)
    }


    pub fn resize_all_cells(
        &mut self,
        w: Option<i32>,
        h: Option<i32>,
        space: &Space<WayfleetWindow>,
    ) -> bool {
        if let Some(width) = w && width <= 0 {
            return false;
        } else if let Some(height) = h && height <= 0 {
            return false;
        }

        let w = w.unwrap_or(self.cell_width);
        let h = h.unwrap_or(self.cell_height);

        // self.cell_width = w.unwrap_or(self.cell_width);
        // self.cell_height = ;

        let mut anim = self.animation.write().unwrap();

        // TODO URGENT: this doesn't work at all, because what if the tile isn't 0x0 rowsxcolumns?
        // maybe fixed?
        let mut size = Size::new(0, 0);
        size.w += w;
        size.h += h;

        let mut done_leaders = HashSet::new();

        for r in 0..self.rows {
            for c in 0..self.columns {
                if let Some(tile) = self.map[r][c].as_ref() {
                    let leader_coord = tile.leader_coord();
                    if done_leaders.contains(&leader_coord) {
                        continue;
                    }
                    done_leaders.insert(leader_coord);

                    if let Err(min_size) = LayoutController::validate_size(&tile.window.window, size) {
                        size = min_size
                    }
                }
            }
        }

        self.cell_width  = size.w;
        self.cell_height = size.h;

        for coord in done_leaders {
            let tile = self[&coord].as_ref().unwrap();
            
            let size = {
                let TileType::Leader { rows, cols, .. } = tile.tile_type else { unreachable!() };

                let w = self.cell_width * (cols as i32 + 1) + self.spaces.horizontal as i32 * cols as i32;
                let h = self.cell_height * (rows as i32 + 1) + self.spaces.vertical as i32 * rows as i32;

                Size::new(w, h)
            };

            anim.schedule::<ResizeAnimation>(
                InfoType::Final(size),
                tile.window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );

            anim.schedule::<MoveAnimation>(
                InfoType::Final(self.get_position_shifted(coord)),
                tile.window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );
        }

        drop(anim);

        self.realign_focused(space);

        true
    }
}
