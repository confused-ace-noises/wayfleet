use std::{
    collections::HashSet, iter, mem, ops::{Deref, DerefMut, Index, IndexMut, Not, Range}, time::{Duration, Instant},
};

use derive_more::{Add, Sub};
use smithay::{
    desktop::{Space, Window},
    reexports::{rustix::net::ipproto::TP, winit::platform::x11::ffi::DontPreferBlanking},
    utils::{Logical, Point, Rectangle, Size},
    wayland::seat::WaylandFocus,
};
use wayfleet_config::{amount::Amount, size::Spaces};

use crate::{layout::{
    animation::{
        Animation, AnimationBase, AnimationHandle, Easing, InfoType, MoveAnimation, ResizeAnimation,
    },
    controller::{LayoutController, ResizeType},
    map::TileType::Regular,
}, state::OutputState};

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
            Regular(coordinate) => coordinate,
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
#[derive(Debug)]
pub struct Map {
    pub map: Vec<Vec<Option<Tile>>>,
    pub first_available: Option<Coordinate>,
    pub rows: usize,
    pub columns: usize,
    pub cell_height: i32,
    pub cell_width: i32,
    pub spaces: Spaces,
    pub offset: Point<i32, Logical>,
    pub animation: AnimationHandle,
}

impl Map {
    pub fn new(
        config: wayfleet_config::Map,
        animation: AnimationHandle,
        OutputState { size: output_size, scale_factor, ..}: &OutputState,
        priv_offset: Point<i32, Logical>
    ) -> Self {
        let wayfleet_config::Map { size, cells, spaces, margins } = config;

        let mut output_size = output_size.to_logical(*scale_factor);

        output_size.h -= priv_offset.y;

        let spaces = spaces.unwrap_or_else(|| Spaces { horizontal: 0, vertical: 0} );

        let (rows, columns) = match size {
            wayfleet_config::size::Size::Specified(wayfleet_config::size::Grid { rows, columns }) => {
                let first = if let Amount::Specified(rows) = rows {
                    rows
                } else {
                    (output_size.h + spaces.vertical as i32) / (cells.unwrap_ref().height.unwrap() + spaces.vertical.max(1) as i32)
                };

                let second = if let Amount::Specified(cols) = columns {
                    cols
                } else {
                    ((output_size.w + spaces.horizontal as i32) as f64 / (cells.unwrap_ref().width.unwrap() + spaces.horizontal.max(1) as i32)as f64) as i32
                };
                
                (
                    first,
                    second
                )
            },
            wayfleet_config::size::Size::Auto => {
                (
                    (output_size.h + spaces.vertical as i32) / (cells.unwrap_ref().height.unwrap() + spaces.vertical.max(1) as i32),
                    ((output_size.w + spaces.horizontal as i32) as f64 / (cells.unwrap_ref().width.unwrap() + spaces.horizontal.max(1) as i32)as f64) as i32
                )
            },
        };

        let (cell_height, cell_width) = match cells {
            wayfleet_config::size::Size::Specified(wayfleet_config::size::SizeRepr { height, width }) => {
                let first = if let Amount::Specified(heigth) = height {
                    heigth
                } else {
                    (output_size.h + spaces.vertical as i32) / rows - spaces.vertical.max(1) as i32
                };

                let second = if let Amount::Specified(width) = width {
                    width
                } else {
                    (output_size.w + spaces.horizontal as i32) / columns - spaces.horizontal.max(1) as i32
                };
                
                (
                    first,
                    second
                )
            },
            wayfleet_config::size::Size::Auto => {
                (
                    (output_size.h + spaces.vertical as i32) / rows - spaces.vertical.max(1) as i32,
                    (output_size.w + spaces.horizontal as i32) / columns - spaces.horizontal.max(1) as i32
                )
            },
        };

        assert_ne!(rows, 0);
        assert_ne!(columns, 0);
        assert!(cell_height > 0);
        assert!(cell_width > 0);

        let columns = columns as usize;
        let rows = rows as usize;

        // TODO: proper margins

        dbg!(Self {
            map: vec![vec![None; columns]; rows],
            first_available: Some([0, 0].into()),
            rows,
            columns,
            cell_height,
            cell_width,
            spaces,
            offset: priv_offset,
            animation,
        })
    }

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

    pub fn swap_or_move(
        &mut self,
        position: &Coordinate,
        direction: Direction,
        space: &mut Space<Window>,
    ) -> Option<bool> {
        let ((g1, mut travel_1), (g2, mut travel_2)) = self.make_swap_groups(*position, direction)?;

        match direction {
            Direction::Up   | Direction::Left  => travel_1 = -travel_1,
            Direction::Down | Direction::Right => travel_2 = -travel_2,
        }

        
        // let (dist_1, dist_2) = match direction {
        //     Direction::Up | Direction::Down => (
        //         Point::<_, Logical>::new(0, travel_1 * self.cell_height + (travel_1 - travel_1.signum()) * self.spaces.vertical as i32),
        //         Point::<_, Logical>::new(0, travel_2 * self.cell_height + (travel_2 - travel_2.signum()) * self.spaces.vertical as i32),
        //     ),
        //     Direction::Left | Direction::Right => (
        //         Point::<_, Logical>::new(travel_1 * self.cell_width + (travel_1 - travel_1.signum()) * self.spaces.horizontal as i32, 0),
        //         Point::<_, Logical>::new(travel_2 * self.cell_width + (travel_2 - travel_2.signum()) * self.spaces.horizontal as i32, 0),
        //     ),
        // };

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

        // println!("g1: {:#?}", g1);
        
        // for Tile { window, .. } in g1.iter() {
        //     println!("g1!");
        //     anim(InfoType::Delta(dist_1), window.clone());
        // }
        
        // println!("g2: {:#?}", g2);

        // for Tile { window, .. } in g2.iter() {
        //     println!("g2!");
        //     anim(InfoType::Delta(dist_2), window.clone());
        // }

        // drop(anim_lock);

        let g1 = g1.into_iter().cloned().collect::<Vec<_>>();
        let g2 = g2.into_iter().cloned().collect::<Vec<_>>();

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
            anim_lock.schedule(Animation::Move(AnimationBase::new(
                result,
                window,
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
                0,
            )));
        };

        println!("has cleared? {:#?}", self.map);
        
        for mut tile in g1 {
            let Tile { tile_type: TileType::Leader { coord, .. }, ..} = &mut tile else { unreachable!() };

            let cloned = *coord + travel_1;

            *coord = cloned;

            anim(InfoType::Final(self.get_position(cloned)), tile.window.clone());
            
            self[&cloned] = Some(tile);
    
            unsafe { self.repoint_regualr_tiles(cloned); }
        }

        println!("g1 insert: {:#?}", self.map);

        for mut tile in g2 {
            let Tile { tile_type: TileType::Leader { coord, .. }, ..} = &mut tile else { unreachable!() };
            
            let cloned = *coord + travel_2;

            *coord = cloned;

            anim(InfoType::Final(self.get_position(cloned)), tile.window.clone());

            self[&cloned] = Some(tile);
    
            unsafe { self.repoint_regualr_tiles(cloned); }
        }

        println!("performed swap: {:#?}", self.map);

        Some(true)
    }

    fn make_swap_groups(
        &self,
        start: Coordinate,
        direction: Direction,
    ) -> Option<((Vec<&Tile>, i32), (Vec<&Tile>, i32))> {
        
        let step: Coordinate = start.step_towards(direction);
        
        // check if we're going outside boundries
        let _ = self.map.get(step.row as usize)?.get(step.column as usize)?;
        
        println!("passed out of bounds check");
        
        let current = self[&start].as_ref()?;
        let leader = self.get_leader(current);
        
        let mut pivot_g1 = dbg!(leader.find_outskirts(self, &direction)[0]);
        let mut pivot_g2 = dbg!(pivot_g1.step_towards(direction));

        let calc_dist = |pivot: &Coordinate, tile: &Tile, current_direction: Direction, is_starting_dir: bool| {
            let farthest = tile.find_outskirts(self, &current_direction)[0];
            
            println!("tile : {:?} has farthest: {farthest:?}", tile.tile_type);

            let dist: Coordinate = *pivot - farthest;
            
            println!("resulting dist: {dist:?}");

            match current_direction {
                Direction::Up | Direction::Down => {
                    dbg!(dist.row + if is_starting_dir { 1 } else { -1 })
                },

                Direction::Left | Direction::Right => dbg!(dist.column + if is_starting_dir { 1 } else { -1 }),
            }
        };

        let calculate_travel = |g: &Vec<&Tile>, pivot: &Coordinate, current_direction: Direction, is_starting_dir: bool| {
            let dist = g.iter()
                .filter_map(|x| match x.tile_type {
                    TileType::Leader { .. } => {                        
                       Some(calc_dist(pivot, x, current_direction, is_starting_dir))
                    },
                    Regular(_) => {
                        println!("none somehow????");
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
                    println!("proj_dist: {}", *projecting_has_to_travel as usize);
                    // 1.
                    tile.project(self, dbg!(Coordinate { row: 0, column: 0 }.step_several(current_direction, *projecting_has_to_travel)))
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
            let new_dist = dbg!(-calculate_travel(&new_blocking, target_pivot, current_direction, current_direction != direction));

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

    fn get_unique_leaders(&self, adj: Vec<Coordinate>) -> Vec<&Tile> {
        adj.into_iter()
            .filter_map(|x| {
                let tile = self[&x].as_ref()?;

                println!("found tile: {tile:?}");

                let coordinate = match tile.tile_type {
                    TileType::Leader { coord, .. } => coord,
                    Regular(coord) => coord,
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
                anim_lock.schedule(Animation::Move(
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
                ));
            }
            // let start = Instant::now();
            anim_lock.schedule(Animation::Resize(
                AnimationBase::<ResizeAnimation>::new_with_time(
                    InfoType::Delta(anim_delta),
                    tile.window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                    start,
                    0,
                ),
            ));

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

            anim_lock.schedule(Animation::Resize(AnimationBase::<ResizeAnimation>::new(
                InfoType::Delta(anim_delta),
                tile.window,
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
                0,
            )));
        }

        unsafe {
            self.move_tile(&coord, &new_coord, space);
        }

        self.recalculate_available();

        Some(true)
    }

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

    pub fn get_leader<'a>(&'a self, tile: &'a Tile) -> &'a Tile {
        match tile.tile_type {
            TileType::Leader { .. } => tile,
            Regular(coordinate) => self[&coordinate].as_ref().unwrap(),
        }
    }

    pub fn get_leader_mut<'a>(&'a mut self, tile: &'a mut Tile) -> &'a mut Tile {
        match tile.tile_type {
            TileType::Leader { .. } => tile,
            Regular(coordinate) => self[&coordinate].as_mut().unwrap(),
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
                        Regular(coordinate) => *coordinate = *new_leader_coord,
                    }
                }

                self[&coord] = popped;
            }
        }

        let mut anim_lock = self.animation.write().unwrap();

        anim_lock.schedule(Animation::Move(AnimationBase::<MoveAnimation>::new(
            InfoType::Final(self.get_position(*new_leader_coord)),
            window,
            space,
            Duration::from_millis(150),
            Easing::EaseInOut,
            0,
        )));
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
            column * self.cell_width + (column-1) * self.spaces.horizontal as i32 + self.offset.x,
            row * self.cell_height + (row - 1) * self.spaces.vertical as i32 + self.offset.y,
        ) + Point::new(1, 1)
    }

    pub fn get_size(&self) -> Size<i32, Logical> {
        Size::new(self.cell_width, self.cell_height)
    }

    pub fn total_width(&self) -> i32 {
        self.cell_width * self.columns as i32 + (self.columns as i32 -1) * self.spaces.horizontal as i32
    }
}

pub enum DoMove {
    Move(Vec<MoveInstructions>),

    NoMove,
}

pub struct MoveInstructions {
    old: Coordinate,
    new: Coordinate,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Not for Direction {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Hash, Add, Sub)]
pub struct Coordinate {
    pub row: i32,
    pub column: i32,
}

impl Coordinate {
    pub fn step_towards(&self, direction: Direction) -> Self {
        *self
            + Into::<Coordinate>::into(match direction {
                Direction::Up => (-1, 0),
                Direction::Down => (1, 0),
                Direction::Left => (0, -1),
                Direction::Right => (0, 1),
            })
    }

    pub fn step_towards_expand(&self, direction: Direction, remove: bool) -> Self {
        *self
            + Into::<Coordinate>::into({
                if remove {
                    match direction {
                        Direction::Up => (0, 0),
                        Direction::Down => (1, 0),
                        Direction::Left => (0, 0),
                        Direction::Right => (0, 1),
                    }
                } else {
                    match direction {
                        Direction::Up => (-1, 0),
                        Direction::Down => (0, 0),
                        Direction::Left => (0, -1),
                        Direction::Right => (0, 0),
                    }
                }
            })
    }

    pub fn step_several(&self, direction: Direction, n: i32) -> Self {
        *self
            + Into::<Coordinate>::into(match direction {
                Direction::Up => (-n, 0),
                Direction::Down => (n, 0),
                Direction::Left => (0, -n),
                Direction::Right => (0, n),
            })
    }
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
