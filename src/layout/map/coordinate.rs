use std::ops::{Index, IndexMut, Not};
use derive_more::{Add, Sub};

use super::{Map, tile::Tile};

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