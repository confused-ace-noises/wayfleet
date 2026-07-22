use smithay::{desktop::{Space, Window}, utils::{Logical, Point}};

use crate::layout::{map::{coordinate::{Coordinate, Direction}, focus::ShiftFocusOutput}, privileged::Privileged};

impl Privileged {
    pub fn new_focus(&mut self, window: &Window, space: &Space<Window>) -> bool {
        let Some(pos) = self.find_position(window) else {
            return false;
        };

        self.new_focus_at(pos, space)
    }

    pub fn new_focus_at(&mut self, pos: (usize, usize), space: &Space<Window>) -> bool {
        if self.is_valid_idxs(pos) {
            self.focused = Some(pos);

            if let Err(to_shift) = self.is_visible(pos) {
                self.shift_all(to_shift, space);
            }

            true
        } else {
            false
        }
    }

    pub fn realign_focused(&mut self, space: &Space<Window>) {
        if let Some(pos) = self.focused 
            && let Err(to_shift) = self.is_visible(pos) 
        {
            self.shift_all(to_shift, space);
        }
    }

    pub fn new_focus_hinted(&mut self, x_hint: i32, space: &Space<Window>) -> Option<Window> {
        let find_last = |point: Point<i32, Logical>| -> Option<&Window> {
            let window = space.element_under(point.to_f64()).map(|x| x.0)?;
            let (column, _) = self.find_position(window)?;
            self.privileged[column].last().map(|x| &x.window)
        };

        let mut point: Point<i32, Logical> = Point::<_, Logical>::new(x_hint, self.viewport.loc.y - 1); // -1 to make sure it falls into a window
        let window = find_last(point).cloned();

        match window {
            Some(found) => self.new_focus(&found, space).then_some(found.clone()),
            None => {
                // maybe we fell in a space in-between windows? retry
                point.x -= self.spaces.horizontal as i32;
                if let Some(new_found) = find_last(point).cloned() {
                    self.new_focus(&new_found, space).then_some(new_found.clone())
                } else {
                    // nope, can't find it. get the first window or just return None
                    let window = self.privileged.first().map(|x| &x[0].window).cloned();

                    if let Some(actual_window) = &window {
                        self.new_focus(actual_window, space);
                    }

                    window
                }
            }
        } 
    }

    pub fn shift_focus(&mut self, direction: Direction, space: &Space<Window>) -> ShiftFocusOutput {
        let Some(mut new_indexes) = self.focused else { return ShiftFocusOutput::Invalid };
        
        // using a wrapping sub because if you're making usize::MAX windows and the
        // validity test below passes that's just on you tbh
        match direction {
            Direction::Up => new_indexes.1 = new_indexes.1.wrapping_sub(1),
            Direction::Down => new_indexes.1 += 1,
            Direction::Left => new_indexes.0 = new_indexes.0.wrapping_sub(1),
            Direction::Right => new_indexes.0 += 1,
        }

        // new index isn't valid
        if !self.is_valid_idxs(new_indexes) {
            if new_indexes.0 == usize::MAX || new_indexes.1 == usize::MAX {
                return ShiftFocusOutput::OutOfBounds;
            }
            // lateral movement resulted in going out of bounds vertically, find last window in 
            // new column
            else if new_indexes.0 < self.privileged.len() && direction != Direction::Down {
                new_indexes.1 = self.privileged[new_indexes.0].len() -1;
                let window = self.privileged[new_indexes.0][new_indexes.1].clone();
                self.new_focus_at(new_indexes, space);
                return ShiftFocusOutput::Success(window)
            } else
            // we went out of bounds downwards or in an unrecoverable way, send hint
            if self.privileged.get(new_indexes.0).map(|x| x.len() <= new_indexes.1).is_some_and(|x| x) && direction == Direction::Down {
                // send hint

                // TODO: figure out if we need to reset the the focused position to None
                return ShiftFocusOutput::OutOfBoundsHinted(self.get_point_raw(Coordinate { column: new_indexes.0 as i32, row: new_indexes.1 as i32 }).x)
            } else {
                return ShiftFocusOutput::OutOfBounds;
            }
        }

        self.new_focus_at(new_indexes, space);
        let window = self.privileged[new_indexes.0][new_indexes.1].clone();
        ShiftFocusOutput::Success(window)

    }
}