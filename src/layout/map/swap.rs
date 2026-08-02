use std::{mem, time::Duration};

use smithay::desktop::Space;

use crate::{animations::{Easing, InfoType, MoveAnimation}, layout::WayfleetWindow};

use super::{Map, coordinate::{Coordinate, Direction}, tile::{Tile, TileType}};

type SwapGroup<'a> = (Vec<&'a Tile>, i32);

impl Map {
    // TODO: add focus tracking
    pub fn swap_or_move_focused(
        &mut self,
        direction: Direction,
        space: &mut Space<WayfleetWindow>,
    ) -> Option<bool> {
        let focused = self.focus?;
        
        self.swap_or_move(&focused, direction, space)
    }

    pub fn swap_or_move(
        &mut self,
        position: &Coordinate,
        direction: Direction,
        space: &mut Space<WayfleetWindow>,
    ) -> Option<bool> {
        let ((g1, mut travel_1), (g2, mut travel_2)) = self.make_swap_groups(*position, direction)?;

        match direction {
            Direction::Up   | Direction::Left  => travel_1 = -travel_1,
            Direction::Down | Direction::Right => travel_2 = -travel_2,
        }

        let (travel_1, travel_2) = match direction {
            Direction::Up | Direction::Down => (
                Coordinate { row: travel_1, column: 0 }, 
                Coordinate { row: travel_2, column: 0 },
            ),
            Direction::Left | Direction::Right => (
                Coordinate { row: 0, column: travel_1 }, 
                Coordinate { row: 0, column: travel_2 },
            ),
        }; 

        let g1 = g1.into_iter().cloned().collect::<Vec<_>>();
        let g2 = g2.into_iter().cloned().collect::<Vec<_>>();

        let is_g2_empty = g2.is_empty();

        for tile in g1.iter().chain(g2.iter())  {
            let Tile { tile_type: TileType::Leader { coord, .. }, ..} = tile else { unreachable!() };
            // SAFETY:
            // the windows *will* get screwed up, but they're gonna be fixed later after the move
            unsafe { self.change_regulars(*coord, None) };
            self[coord] = None;
        }

        let handle = self.animation.clone();
        let mut anim_lock = handle.write().unwrap();

        let mut anim = |result, window| {
            anim_lock.schedule::<MoveAnimation>(
                result,
                window,
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );
        };
        
        for mut tile in g1 {
            let Tile { tile_type: TileType::Leader { coord, .. }, ..} = &mut tile else { unreachable!() };

            let cloned = *coord + travel_1;

            *coord = cloned;

            anim(InfoType::Final(self.get_position_shifted(cloned)), tile.window.clone());
            
            self[&cloned] = Some(tile);
    
            unsafe { self.repoint_regualr_tiles(cloned); }
        }

        for mut tile in g2 {
            let Tile { tile_type: TileType::Leader { coord, .. }, ..} = &mut tile else { unreachable!() };
            
            let cloned = *coord + travel_2;

            *coord = cloned;

            anim(InfoType::Final(self.get_position_shifted(cloned)), tile.window.clone());

            self[&cloned] = Some(tile);
    
            unsafe { self.repoint_regualr_tiles(cloned); }
        }

        if is_g2_empty {
            self.recalculate_available();
        }

        self.shift_focus(direction, space);

        Some(true)
    }

    fn make_swap_groups<'a>(
        &'a self,
        start: Coordinate,
        direction: Direction,
    ) -> Option<(SwapGroup<'a>, SwapGroup<'a>)> {
        
        let step: Coordinate = start.step_towards(direction);
        
        // check if we're going outside boundries
        let _ = self.map.get(step.row as usize)?.get(step.column as usize)?;
                
        let current = self[&start].as_ref()?;
        let leader = self.get_leader(current);
        
        let mut pivot_g1 = leader.find_outskirts(self, &direction)[0];
        let mut pivot_g2 = pivot_g1.step_towards(direction);

        let calc_dist = |pivot: &Coordinate, tile: &Tile, current_direction: Direction, is_starting_dir: bool| {
            let farthest = tile.find_outskirts(self, &current_direction)[0];
            
            let dist: Coordinate = *pivot - farthest;

            match current_direction {
                Direction::Up | Direction::Down => {
                    dist.row + if is_starting_dir { 1 } else { -1 }
                },

                Direction::Left | Direction::Right => dist.column + if is_starting_dir { 1 } else { -1 },
            }
        };

        let calculate_travel = |g: &Vec<&Tile>, pivot: &Coordinate, current_direction: Direction, is_starting_dir: bool| {
            let dist = g.iter()
                .filter_map(|x| match x.tile_type {
                    TileType::Leader { .. } => {                        
                       Some(calc_dist(pivot, x, current_direction, is_starting_dir))
                    },
                    TileType::Regular(_) => {
                        None
                    },
                })
                .fold(0, |mut acc: i32, x| {
                    if x.abs() > acc.abs() {
                        acc = x;
                    }

                    acc
                });
                
            if dist == 0 { // this can onluy happen on g2, and if g2 really does no have any members, just treat it like a move
                1
            } else {
                dist
            }
        };

        let TileType::Leader { rows: root_rows, cols: root_cols, .. } = leader.tile_type else { unreachable!() };

        let mut g1: Vec<&Tile> = Vec::new();
        let mut g2: Vec<&Tile> = Vec::new();
        
        g1.push(leader);

        let mut current_projecting = &mut g1;
        let mut current_target = &mut g2;

        let mut projecting_pivot = &mut pivot_g1;
        let mut target_pivot = &mut pivot_g2;
     
        let mut g1_dist: i32 = 1;
        let mut g2_dist = match direction {
            Direction::Up | Direction::Down => root_rows + 1,
            Direction::Left | Direction::Right => root_cols + 1,
        } as i32 ;
        
        let mut projecting_has_to_travel = &mut g1_dist;
        let mut target_has_to_travel = &mut g2_dist;
        
        let mut current_direction = direction;
        
        let mut finish = false;
        
        
        loop {
            // 1. make bounding box
            // 2. check if movement is ok
            // 3. if not, change dist until it is
            // 4. swap groups, repeat

            let new_blocking = {
                current_projecting.iter().map(|tile| {
                    // 1.
                    tile.project(self, Coordinate { row: 0, column: 0 }.step_several(current_direction, *projecting_has_to_travel))
                }).fold(Vec::new(), |mut acc: Vec<&Tile>, tiles| {
                    for tile in tiles {
                        if !acc.contains(&tile) && !current_target.contains(&tile) {
                            acc.push(tile);
                            current_target.push(tile);
                        }
                    }
                
                    acc
                })
            };
            
            // 2.
            let new_dist = -calculate_travel(&new_blocking, target_pivot, current_direction, current_direction != direction);

            // 3.
            if new_dist.abs() > projecting_has_to_travel.abs() {
                *projecting_has_to_travel = new_dist;
                finish = false;
            } else {
                if finish {
                    break;
                }

                finish = true;
            }

            // 4.
            current_direction = !current_direction;
            mem::swap(&mut current_projecting, &mut current_target);
            mem::swap(&mut projecting_has_to_travel, &mut target_has_to_travel);
            mem::swap(&mut projecting_pivot, &mut target_pivot);
        }   

        Some(((g1, g1_dist), (g2, g2_dist)))
    }
}