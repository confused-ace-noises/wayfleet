use std::{
    sync::{Arc, RwLock}, time::Duration,
};

use smithay::{
    desktop::{Space, Window},
    utils::Rectangle,
};

use crate::{
    animations::{
        Easing,
        InfoType, MoveAnimation,
    },
    layout::{
        IntoWayfleetWindow, WayfleetWindow, WayfleetWindowType,
        controller::{LayoutController, ResizeType},
        privileged::{Privileged, tile::Tile},
    },
};

impl Privileged {
    pub fn new_insert_soft_last(
        &mut self,
        window: Window,
        space: &mut Space<WayfleetWindow>,
    ) -> (WayfleetWindow, Option<i32>) {
        let len = self.privileged.len();

        self.new_insert_soft(len, window, space)
    }

    pub fn new_insert_soft(
        &mut self,
        column: usize,
        window: Window,
        space: &mut Space<WayfleetWindow>,
    ) -> (WayfleetWindow, Option<i32>) {
        let mut size = self.std_size;

        let mut new_map_offset = None;

        if let Err(min) = LayoutController::validate_size(&window, size) {
            if size.h < min.h {
                let offset = self.set_height_all_cells(min.h, space);
                new_map_offset = Some(offset);
                size.h = min.h;
            }

            if size.w < min.w {
                size.w = min.w;
            }
        }

        let pos = self.get_point_tuple_shifted((column, 0));

        let wayfleet_window = window.as_priv(Arc::new(RwLock::new(Rectangle { loc: pos, size })), self.viewport.clone());

        LayoutController::resize(&window, ResizeType::Both(size));
        window.set_activated(true);

        {
            let mut anim = self.animation.write().unwrap();

            anim.schedule::<MoveAnimation>(
                InfoType::Final(pos),
                wayfleet_window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );
        }

        // space.map_element(wayfleet_window.clone(), pos, true);
        self.shift_columsn(size.w + self.spaces.horizontal as i32, column.., space);
        self.privileged.insert(
            column,
            vec![Tile {
                window: wayfleet_window.clone(),
                size,
            }],
        );

        self.new_focus_at((column, 0), space);

        (wayfleet_window, new_map_offset)
    }

    pub fn new_insert_right_of_focus(
        &mut self,
        window: Window,
        space: &mut Space<WayfleetWindow>,
    ) -> (WayfleetWindow, Option<i32>) {
        self.focused
            .map(|(focused_col, _)| self.new_insert(focused_col + 1, window.clone(), space))
            .unwrap_or_else(|| self.new_insert_last(window, space))
    }

    pub fn new_insert_last(
        &mut self,
        window: Window,
        space: &mut Space<WayfleetWindow>,
    ) -> (WayfleetWindow, Option<i32>) {
        let len = self.privileged.len();

        self.new_insert(len, window, space)
    }

    pub fn new_insert(
        &mut self,
        column: usize,
        window: Window,
        space: &mut Space<WayfleetWindow>,
    ) -> (WayfleetWindow, Option<i32>) {
        let mut size = self.std_size;

        let mut new_map_offset = None;

        if let Err(min) = LayoutController::validate_size(&window, size) {
            if size.h < min.h {
                let offset = self.set_height_all_cells(min.h, space);
                new_map_offset = Some(offset);
                size.h = min.h;
            }

            if size.w < min.w {
                size.w = min.w;
            }
        }

        let pos = self.get_point_tuple_shifted((column, 0));

        let wayfleet_window = window.as_priv(self.viewport.clone(), Arc::new(RwLock::new(Rectangle { loc: pos, size })));

        LayoutController::resize(&window, ResizeType::Both(size));

        space.map_element(wayfleet_window.clone(), pos, true);
        self.shift_columsn(size.w + self.spaces.horizontal as i32, column.., space);
        self.privileged.insert(
            column,
            vec![Tile {
                window: wayfleet_window.clone(),
                size,
            }],
        );

        self.new_focus_at((column, 0), space);

        (wayfleet_window, new_map_offset)
    }

    pub fn remove(&mut self, window: Window, space: &mut Space<WayfleetWindow>) {
        let Some((column_idx, idx)) = self.find_position_window(&window) else {
            return;
        };

        let window = WayfleetWindow {
            window,
            window_type: WayfleetWindowType::Privileged(self.viewport.clone()),
            specific_crop: Arc::new(RwLock::new(Rectangle::zero())),
            is_focused: Arc::new(RwLock::new(false)),
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

            if let Some((focused_col, _)) = &mut self.focused
                && *focused_col > column_idx
            {
                *focused_col -= 1;
            }

            self.realign_focused(space);
            space.unmap_elem(&window);
            self.shift_columsn(
                -(size.w + self.spaces.horizontal as i32),
                column_idx..,
                space,
            );
        }
    }

    pub fn soft_remove(&mut self, window: Window, space: &Space<WayfleetWindow>) {
        let Some((column_idx, idx)) = self.find_position_window(&window) else {
            return;
        };

        let column = &self.privileged[column_idx];

        if column.len() > 1 {
            // need to recalculate the vertical space taken up
            let Tile { size, .. } = self.privileged[column_idx].remove(idx);

            // space.unmap_elem(&window);
            self.recalc_heights(column_idx, size.h + self.spaces.vertical as i32, space);
        } else {
            // need to remove the window and move the left ones to the right
            let Tile { size, .. } = self.privileged.remove(column_idx).remove(0);

            if let Some((focused_col, _)) = &mut self.focused
                && *focused_col > column_idx
            {
                *focused_col -= 1;
            }

            self.realign_focused(space);
            // space.unmap_elem(&window);
            self.shift_columsn(
                -(size.w + self.spaces.horizontal as i32),
                column_idx..,
                space,
            );
        }
    }
}
