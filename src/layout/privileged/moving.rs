use std::time::Duration;

use smithay::{
    desktop::{Space, Window},
    utils::{Logical, Point, Size},
};

use crate::{
    animations::{Easing, InfoType, MoveAnimation, ResizeAnimation},
    layout::map::coordinate::Direction,
};

use super::Privileged;

impl Privileged {
    pub fn swap_focused(&mut self, 
        direction: Direction, 
        space: &mut Space<Window>
    ) -> Option<()> {
        let focused = self.focused?;

        self.swap(focused, direction, space);
        Some(())
    }

    pub fn swap(
        &mut self,
        (column_idx, idx): (usize, usize),
        direction: Direction,
        space: &mut Space<Window>,
    ) {
        let mut other_column = column_idx;
        let mut other_idx = idx;

        match direction {
            Direction::Up => other_idx -= 1,
            Direction::Down => other_idx += 1,
            Direction::Left => other_column -= 1,
            Direction::Right => other_column += 1,
        }

        if column_idx != other_column {
            // swap columns

            // TODO: this needs to be redone so that it
            // accounts for different sized columns
            let reference1 = self.get_point_tuple_shifted((column_idx, 0));
            let reference2 = self.get_point_tuple_shifted((other_column, 0));

            let [col1, col2] = [
                self
                    .privileged
                    .get(column_idx)
                    .unwrap(), 
                self
                    .privileged
                    .get(other_column)
                    .unwrap()
            ];

            let mut animation = self.animation.write().unwrap();

            for (n, tile) in col1.iter().enumerate() {
                let point = self.get_point_tuple_shifted((column_idx, n));

                animation.schedule::<MoveAnimation>(
                    InfoType::Final(Point::new(reference2.x, point.y)),
                    tile.window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );
            }

            for (n, tile) in col2.iter().enumerate() {
                let point = self.get_point_tuple_shifted((other_column, n));

                animation.schedule::<MoveAnimation>(
                    InfoType::Final(Point::new(reference1.x, point.y)),
                    tile.window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );
            }

            self.privileged.swap(column_idx, other_column);
        } else {
            // swap windows within column
            let column = &self.privileged[column_idx];
            let win1 = column.get(idx).unwrap();
            let win2 = column.get(other_idx).unwrap();

            // TODO: this needs to be redone so that it
            // accounts for different sized windows
            let mut win1_pos = self.get_point_tuple_shifted((column_idx, idx));
            let mut win2_pos = self.get_point_tuple_shifted((column_idx, other_idx));

            // also TODO, fix this
            
            // if idx == 0 || other_idx == 0 {
            //     win1_pos.y += self.spaces.vertical as i32;
            //     win2_pos.y -= self.spaces.vertical as i32;
            // }

            let mut animation = self.animation.write().unwrap();

            animation.schedule::<MoveAnimation>(
                InfoType::Final(win2_pos),
                win1.window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );

            animation.schedule::<MoveAnimation>(
                InfoType::Final(win1_pos),
                win2.window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );

            let column = &mut self.privileged[other_column];

            column.swap(idx, other_idx);
        }

        if let Some((col, idx)) = self.focused.as_mut() {
            match direction {
                Direction::Up    => *idx -= 1,
                Direction::Down  => *idx += 1,
                Direction::Left  => *col -= 1,
                Direction::Right => *col += 1,
            }
        }
        
        self.realign_focused(space);
    }

    pub fn swap_window(
        &mut self,
        window: &Window,
        direction: Direction,
        space: &mut Space<Window>,
    ) {
        let Some((column_idx, idx)) = self.find_position(window) else {
            return;
        };

        self.swap((column_idx, idx), direction, space);
    }

    pub fn push_laterally(
        &mut self,
        (column_idx, idx): (usize, usize),
        direction: Direction,
        space: &mut Space<Window>,
    ) -> bool {
        let (Direction::Left | Direction::Right) = direction else {
            unimplemented!("not lateral movement")
        };

        if self.privileged[column_idx].len() == 1 {
            // window needs to be absorbed

            let new_column_idx = match direction {
                Direction::Left => {
                    if column_idx == 0 {
                        return false;
                    }
                    column_idx - 1
                }
                Direction::Right => {
                    if column_idx == self.privileged.len() - 1 {
                        return false;
                    }
                    column_idx
                }
                _ => unreachable!(),
            };

            let mut to_be_absorbed = self.privileged.remove(column_idx).remove(0);

            let destination_len_old = self.privileged[new_column_idx].len() as i32;

            // give rough height first, get back the actual new space available after
            let rough_height = self.viewport.size.h / (destination_len_old + 1) as i32;

            let actual_height = -self.recalc_heights(
                new_column_idx,
                -(rough_height + self.spaces.vertical as i32),
                space,
            ) - self.spaces.vertical as i32;

            // size
            let width = self.privileged[new_column_idx][0].size.w;
            let size: Size<i32, Logical> = Size::new(width, actual_height);
            let old_size = to_be_absorbed.size;
            to_be_absorbed.size = size;

            let pos = self.get_point_tuple_shifted((new_column_idx, destination_len_old as usize));

            let window = to_be_absorbed.window.clone();
            self.privileged[new_column_idx].push(to_be_absorbed);

            {
                let mut anim = self.animation.write().unwrap();
                anim.schedule::<MoveAnimation>(
                    InfoType::Final(pos),
                    window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );

                anim.schedule::<ResizeAnimation>(
                    InfoType::Final(size),
                    window,
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );
            }

            self.shift_columsn(
                -(old_size.w + self.spaces.horizontal as i32),
                (new_column_idx + 1)..,
                space,
            );

            self.focused = Some((new_column_idx, destination_len_old as usize));
            self.realign_focused(space);

            true
        } else {
            // window needs to be expelled
            let new_column_idx = match direction {
                Direction::Left => column_idx,
                Direction::Right => column_idx + 1,
                _ => unreachable!(),
            };

            let mut to_expel = self.privileged[column_idx].remove(idx);

            self.recalc_heights(
                column_idx,
                to_expel.size.h + self.spaces.vertical as i32,
                space,
            );

            self.shift_columsn(
                to_expel.size.w + self.spaces.horizontal as i32,
                new_column_idx..,
                space,
            );

            let size = Size::new(to_expel.size.w, self.viewport.size.h);

            to_expel.size = size;

            let window = to_expel.window.clone();

            self.privileged.insert(new_column_idx, vec![to_expel]);
            
            let pos = self.get_point_tuple_shifted((new_column_idx, 0));
            
            {
                let mut anim = self.animation.write().unwrap();
                
                anim.schedule::<MoveAnimation>(
                    InfoType::Final(pos),
                    window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );
                
                anim.schedule::<ResizeAnimation>(
                    InfoType::Final(size),
                    window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );
            }
            
            self.focused = Some((new_column_idx, 0));
            self.realign_focused(space);

            true
        }
    }

    pub fn push_focus_laterally(
        &mut self,
        direction: Direction,
        space: &mut Space<Window>,
    ) -> bool {
        if let Some(focus) = self.focused {
            self.push_laterally(focus, direction, space);
            true
        } else {
            false
        }
    }
}
