use std::{
    ops::{Index, IndexMut, RangeBounds}, slice::SliceIndex, time::Duration,
};

use smithay::{
    desktop::{Space, Window},
    utils::{Logical, Point},
};

use super::Privileged;
use crate::{
    animations::{Easing, InfoType, MoveAnimation},
    layout::{map::coordinate::Coordinate, privileged::tile::Tile},
};

impl Index<Coordinate> for Privileged {
    type Output = Tile;

    fn index(&self, index: Coordinate) -> &Self::Output {
        &self.privileged[index.column as usize][index.row as usize]
    }
}

impl IndexMut<Coordinate> for Privileged {
    fn index_mut(&mut self, index: Coordinate) -> &mut Self::Output {
        &mut self.privileged[index.column as usize][index.row as usize]
    }
}

impl Privileged {
    pub fn shift_columsn<T>(&mut self, delta: i32, range: T, space: &Space<Window>)
    where
        T: RangeBounds<usize> + SliceIndex<[Vec<Tile>]>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(x) => *x,
            std::ops::Bound::Excluded(x) => *x + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(x) => *x,
            std::ops::Bound::Excluded(x) => *x - 1,
            std::ops::Bound::Unbounded => {
                if self.privileged.is_empty()  {
                    return;
                } else {
                    self.privileged.len()-1
                }
            },
        };

        let mut anim = self.animation.write().unwrap();

        for col_idx in start..=end {
            for tile in &self.privileged[col_idx] {
                anim.schedule::<MoveAnimation>(
                    InfoType::Delta(Point::new(delta, 0)),
                    tile.window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );
            }
        }
    }

    pub fn shift_all(&mut self, delta: i32, space: &Space<Window>) {
        self.right_shift -= delta;
        self.shift_columsn(delta, .., space);
    }

    pub fn is_visible(&self, (column, idx): (usize, usize)) -> Result<(), i32> {
        let point_left = self.get_point_tuple_raw((column, idx));

        let rect = {
            let mut area = self.viewport;
            area.loc.x += self.right_shift;
            area
        };

        let point_right = point_left + Point::new(self.privileged[column][idx].size.w, 0);

        match (rect.contains(point_left), rect.contains(point_right)) {
            (true, true) => Ok(()),
            (true, false) => {
                let rect_top_right = rect.loc.x + rect.size.w;

                dbg!(Err(rect_top_right - point_right.x))
            }
            (false, true) => {
                let rect_top_left = rect.loc.x;

                Err(rect_top_left - point_left.x)
            }

            (false, false) => {
                // outisde on the left
                if point_left.x < rect.loc.x {
                    let rect_top_left = rect.loc.x;

                    Err(rect_top_left - point_left.x)
                } else {
                    // outside on the right
                    let rect_top_right = rect.loc.x + rect.size.w;

                    Err(rect_top_right - point_right.x)
                }
            }
        }
    }

    pub fn find_position(&self, searching: &Window) -> Option<(usize, usize)> {
        self.privileged
            .iter()
            .enumerate()
            .find_map(|(col_idx, column)| {
                column.iter().enumerate().find_map(|(win_idx, tile)| {
                    (tile.window == *searching).then_some(Some((col_idx, win_idx)))
                })
            })
            .flatten()
    }

    pub fn is_valid_idxs(&self, (column, idx): (usize, usize)) -> bool {
        column < self.privileged.len() && idx < self.privileged[column].len()
    }

    /// TODO: check if it works
    pub fn get_point_raw(&self, coord: Coordinate) -> Point<i32, Logical> {
        let Coordinate {
            row: idx,
            column: column_idx,
        } = coord;

        self.get_point_tuple_raw((column_idx as usize, idx as usize))
    }

    pub fn get_point_tuple_raw(&self, (column_idx, idx): (usize, usize)) -> Point<i32, Logical> {
        let mut current: Point<_, Logical> = Point::new(0, 0);

        for col in 0..column_idx {
            let tile = &self.privileged[col][0];

            current.x += tile.size.w + self.spaces.horizontal as i32;
        }

        for row in 0..idx {
            let tile = &self.privileged[column_idx][row];

            current.y += tile.size.h + self.spaces.vertical as i32;
        }

        current
    }

    pub fn get_point_tuple_shifted(&self, pos: (usize, usize)) -> Point<i32, Logical> {
        self.get_point_tuple_raw(pos) - Point::new(self.right_shift, 0)
    }

    pub fn radial_search<'a>(&'a self, searching: &Window) -> Option<&'a Window> {
        let (column_idx, idx) = self.find_position(searching)?;

        let current_column = &self.privileged[column_idx];

        if idx < current_column.len() - 1 {
            // select the window in the same column, under
            Some(&current_column[idx + 1])
        } else if current_column.len() - 1 == idx  && idx != 0 {
            // same column, upper one
            Some(&current_column[idx - 1])
        } else {
            // there's only the window we're searching from in this column, try left/right columns
            if column_idx > 0 {
                // there's a window to the left
                Some(&self.privileged[column_idx - 1][0])
            } else if self.privileged.len() > 1 {
                // there's a window to the right, because the one deleted is at idx 0
                Some(&self.privileged[column_idx + 1][0])
            } else {
                // this window is the only one in the privileged area.
                None
            }
        }
    }
}