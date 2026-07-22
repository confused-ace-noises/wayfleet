use smithay::{desktop::{Space, Window}, utils::{Logical, Point}};

use crate::layout::{controller::{LayoutController, ResizeType}, privileged::{Privileged, tile::Tile}};

impl Privileged {
    pub fn insert_right_of_focus(&mut self, window: Window, space: &Space<Window>) -> Option<Point<i32, Logical>> {
        self.focused.map(|(focused_col, _)| self.insert_new(focused_col+1, window, space))
    }

    pub fn insert_new_last(&mut self, window: Window, space: &Space<Window>) -> Point<i32, Logical> {
        let len = self.privileged.len();

        self.insert_new(len, window, space)
    }
    
    pub fn insert_new(&mut self, column: usize, window: Window, space: &Space<Window>) -> Point<i32, Logical> {
        let size = self.std_size;
        
        // dont animate because the resize should be instant here
        LayoutController::resize(&window, ResizeType::Both(size));

        // make space
        self.shift_columsn(size.w + self.spaces.horizontal as i32, column.., space);

        self.privileged.insert(column, vec![Tile { window, size }]);

        self.get_point_tuple_shifted((column, 0))
    }

    pub fn remove(&mut self, window: Window, space: &mut Space<Window>) {
        let Some((column_idx, idx)) = self.find_position(&window) else {
            return;
        };

        let column = &self.privileged[column_idx];

        if column.len() > 1 {
            // need to recalculate the vertical space taken up
            let Tile { size, .. } = self.privileged[column_idx].remove(idx);

            space.unmap_elem(&window);
            self.recalc_heights(column_idx, size.h + self.spaces.vertical as i32, space);
        } else {
            // need to remove the window and move the left ones to the right
            let Tile { size, .. } = self.privileged.remove(column_idx).remove(0);
            
            if let Some((focused_col, _)) = &mut self.focused && *focused_col > column_idx {
                *focused_col-=1;
            }

            self.realign_focused(space);
            space.unmap_elem(&window);
            self.shift_columsn(-(size.w + self.spaces.horizontal as i32), column_idx.., space);
        }
    }
}