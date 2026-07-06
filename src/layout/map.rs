use std::{
    collections::HashSet,
    mem,
    ops::{Deref, DerefMut, Index, IndexMut, Range},
    time::{Duration, Instant},
};

use derive_more::Add;
use smithay::{
    desktop::{Space, Window},
    reexports::{rustix::net::ipproto::TP, winit::platform::x11::ffi::DontPreferBlanking},
    utils::{Logical, Point, Rectangle, Size},
    wayland::seat::WaylandFocus,
};

use crate::layout::{
    animation::{Animation, AnimationBase, AnimationHandle, Easing, InfoType, MoveAnimation, ResizeAnimation}, controller::{LayoutController, ResizeType}, map::TileType::Regular,
};

#[derive(Debug, Clone)]
pub struct Tile {
    pub window: Window,
    pub tile_type: TileType,
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
    pub offset: Point<i32, Logical>,
    pub animation: AnimationHandle,
}

impl Map {
    pub fn new(
        rows: usize,
        columns: usize,
        cell_height: i32,
        cell_width: i32,
        offset: Point<i32, Logical>,
        animation: AnimationHandle,
    ) -> Self {
        // TODO tmp comment!!
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
            animation,
        }
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
                rows -= 1;
                anim_delta.h += -self.cell_height;
            } else {
                cols -= 1;
                anim_delta.w += -self.cell_width;
            }

            let start = Instant::now();

            if new_coord != coord {
                anim_lock.schedule(Animation::Move(AnimationBase::<MoveAnimation>::new_with_time(
                    InfoType::Final(self.get_position(new_coord)),
                    tile.window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                    start,
                    // this is just a magic number, i thought it should
                    // be 1 but it works with 2 for some reason? idk
                    2 
                )));
                // space.relocate_element(&tile.window, self.get_position(new_coord));
            }
            // let start = Instant::now();
            anim_lock.schedule(Animation::Resize(AnimationBase::<ResizeAnimation>::new_with_time(
                InfoType::Delta(anim_delta),
                tile.window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
                start,
                0
            )));

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

        let Tile {
            tile_type: TileType::Leader { rows, cols, .. },
            ..
        } = self[&new_coord].as_mut().unwrap()
        else {
            unreachable!()
        };

        let anim_delta: Size<i32, Logical>;

        if let Direction::Down | Direction::Up = direction {
            anim_delta = Size::new(0, cell_height);
            *rows += 1;
        } else {
            anim_delta = Size::new(cell_width, 0);
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
                0
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
            tile_type: TileType::Leader { rows, cols, coord },
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
            0
        )));
    }

    /// # Safety
    /// forceful function, could break some windows
    ///
    /// # Panics
    /// will panic if leader isn't actually a leader.
    pub unsafe fn repoint_regualr_tiles(&mut self, leader: Coordinate) {
        let Some(Tile {
            tile_type: TileType::Leader { rows, cols, .. },
            window,
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

        let window = window.clone();

        for r in leader.row..last.row {
            for c in leader.column..last.column {
                if first {
                    first = false;
                    continue;
                }

                self[&(r, c).into()] = Some(Tile::new_regular(window.clone(), leader))
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

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {}

#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Hash, Add)]
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
