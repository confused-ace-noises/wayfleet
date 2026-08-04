use std::time::Duration;

use smithay::{
    desktop::Space, utils::{Logical, Point, Size},
};

use crate::{
    animations::{Easing, InfoType, MoveAnimation, ResizeAnimation}, layout::{
        WayfleetWindow,
        privileged::{Privileged, recalc::split_evenly},
    }, state::CONFIG,
};

impl Privileged {
    pub fn set_height_all_cells_delta(&mut self, delta: i32, space: &Space<WayfleetWindow>) -> i32 {
        self.map_offset += delta;
        self.viewport.write().unwrap().size.h += delta;
        self.std_size.h += delta;

        for column in 0..self.privileged.len() {
            self.recalc_heights(column, delta, space);
        }

        self.map_offset
    }

    pub fn set_height_all_cells(&mut self, h: i32, space: &Space<WayfleetWindow>) -> i32 {
        let config = CONFIG.get().unwrap();
        
        let delta = h  + config.layout.privileged.padding.down - self.map_offset;

        if h - config.layout.privileged.padding.down <= 0 {
            return self.map_offset;
        }

        self.map_offset = h + config.layout.privileged.padding.down; // + config.layout.privileged.padding.down + config.layout.map.padding.top;
        self.viewport.write().unwrap().size.h = h;
        self.std_size.h = h;

        for column in 0..self.privileged.len() {
            self.recalc_heights(column, delta, space);
        }

        self.map_offset
    }

    pub fn resize_column(&mut self, column: usize, delta: i32, space: &Space<WayfleetWindow>) {
        let mut anim = self.animation.write().unwrap();

        let mut size = Size::new(0, 0);
        size.w += delta;

        for tile in self.privileged[column].iter_mut() {
            tile.size.w += delta;

            anim.schedule::<ResizeAnimation>(
                InfoType::Delta(size),
                tile.window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );
        }

        drop(anim);
        self.shift_columsn(delta, (column + 1).., space);
    }

    pub fn resize_window(
        &mut self,
        (column, pos): (usize, usize),
        delta: i32,
        space: &Space<WayfleetWindow>,
    ) {
        let column = &mut self.privileged[column];

        if column.len() == 1 {
            return;
        }

        let mut deltas = split_evenly(delta, column.len() as i32 - 2).peekable();

        let mut anim = self.animation.write().unwrap();

        let mut accumulated = 0;

        for (idx, tile) in column.iter_mut().enumerate() {
            if accumulated != 0 {
                anim.schedule::<MoveAnimation>(
                    InfoType::Delta(Point::new(0, accumulated)),
                    tile.window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );
            }

            let mut size = Size::<_, Logical>::new(0, 0);

            if idx != pos {
                let current_delta = deltas.next().unwrap();
                
                size.h += current_delta;
                accumulated += current_delta;
            } else {
                size.h += delta;
            }

            anim.schedule::<ResizeAnimation>(
                InfoType::Delta(size),
                tile.window.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );

            tile.size += size;
        }
    }
}
