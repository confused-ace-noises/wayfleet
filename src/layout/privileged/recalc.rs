use std::time::Duration;

use smithay::{
    desktop::{Space, Window},
    utils::{Point, Size},
};

use crate::{
    animations::{Easing, InfoType, MoveAnimation, ResizeAnimation}, layout::privileged::Privileged,
};

impl Privileged {
    pub fn recalc_widths(&mut self, total_delta: i32, space: &Space<Window>) {
        let columns = self.privileged.len() as i32;
        if columns == 0 {
            return;
        }

        let deltas = split_evenly(total_delta, columns);
        let mut animation = self.animation.write().unwrap();
        let mut pos = 0;

        for (col, delta) in self.privileged.iter_mut().zip(deltas) {
            let mut pos_y = 0;
            for tile in col.iter_mut() {
                tile.size.w += delta;
                let mut size = Size::new(0, 0);

                size.w += delta;

                animation.schedule::<ResizeAnimation>(
                    InfoType::Delta(size), 
                    tile.clone(), 
                    space, 
                    Duration::from_millis(150), 
                    Easing::EaseInOut
                );

                animation.schedule::<MoveAnimation>(
                    InfoType::Final(Point::new(pos, pos_y)),
                    tile.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::EaseInOut,
                );

                pos_y += tile.size.h + self.spaces.vertical as i32;
            }
            pos += col[0].size.w + self.spaces.horizontal as i32;
        }
    }

    /// call BEFORE adding
    /// call AFTER removing
    /// returns actual delta
    pub fn recalc_heights(&mut self, column: usize, total_delta: i32, space: &Space<Window>) -> i32 {
        let pos_x: i32 = self.privileged.iter().take(column).map(|x| x[0].size.w + self.spaces.horizontal as i32).sum::<i32>() - self.right_shift;
        
        let column = &mut self.privileged[column];
        let len = column.len() as i32;

        if len == 0 {
            return 0;
        }

        let deltas = split_evenly(total_delta, len);

        let mut animation = self.animation.write().unwrap();
        let mut pos = 0;
        let mut tot = 0;

        for (tile, delta) in column.iter_mut().zip(deltas) {
            tile.size.h += delta;

            let mut size = Size::new(0, 0);
            size.h += delta;

            animation.schedule::<ResizeAnimation>(
                InfoType::Delta(size),
                (*tile).clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );

            animation.schedule::<MoveAnimation>(
                InfoType::Final(Point::new(pos_x, pos)),
                (*tile).clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
            );

            tot += delta;
            pos += tile.size.h + self.spaces.vertical as i32;
        } 

        tot
    }
}

/// splits accounting for integer division rounding
fn split_evenly(total: i32, n: i32) -> impl Iterator<Item = i32> {
    let base = total / n;
    let rem = ((total * total.signum()) % n) as usize;

    std::iter::repeat_n(base + 1, rem)
        .chain(std::iter::repeat_n(base, n as usize - rem))
}