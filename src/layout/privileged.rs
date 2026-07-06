use std::time::Duration;

use smithay::{
    desktop::{Space, Window},
    input::keyboard::Layout,
    utils::{Logical, Point, Rectangle},
};

use crate::layout::{
    animation::{Animation, AnimationBase, AnimationHandle, Easing, InfoType, MoveAnimation},
    controller::{LayoutController, ResizeType},
    map::Direction,
};

pub struct Privileged {
    pub area: Rectangle<i32, Logical>,
    pub privileged: Vec<Vec<Window>>,
    pub animation: AnimationHandle,
}

impl Privileged {
    pub fn new(area: Rectangle<i32, Logical>, animation: AnimationHandle) -> Self {
        Self {
            privileged: vec![],
            area,
            animation,
        }
    }

    pub fn insert(&mut self, window: Window, space: &mut Space<Window>) -> Point<i32, Logical> {
        let mut size = self.area.size;

        size.w /= self.privileged.len() as i32 + 1;

        self.redo_widths(space, -size.w);
        self.privileged.push(vec![window.clone()]);

        LayoutController::resize(&window, ResizeType::Both(size));

        Point::new(self.area.size.w - size.w + self.area.loc.x, self.area.loc.y)
    }

    /// AFTER remove
    /// BEFORE add
    pub fn redo_widths(&self, space: &mut Space<Window>, available_width: i32) {
        let columns = self.privileged.len() as i32;

        if columns == 0 {
            return;
        }

        let single_width_delta = available_width / (columns);

        for (n_col, column) in self.privileged.iter().enumerate() {
            for window in column.iter() {
                LayoutController::resize_delta(window, ResizeType::Width(single_width_delta));

                let mut pos = space
                    .element_location(window)
                    .expect("well this shouldn't happen");
                pos.x += single_width_delta * n_col as i32;
                space.relocate_element(window, pos);
            }
        }
    }

    /// AFTER remove
    /// BEFORE add
    pub fn redo_height(&self, space: &mut Space<Window>, gained_height: i32, column_idx: usize) {
        let column = &self.privileged[column_idx];

        if column.is_empty() {
            return;
        }

        let delta = gained_height / column.len() as i32;

        for (idx, window) in column.iter().enumerate() {
            LayoutController::resize_delta(window, ResizeType::Height(delta));

            let mut pos = space
                .element_location(window)
                .expect("well this shouldn't happen");
            pos.y += delta * idx as i32;
            space.relocate_element(window, pos);
        }
    }

    pub fn swap_window(&mut self, window: &Window, space: &mut Space<Window>, direction: Direction) {
        let Some((column_idx, idx)) = self.find_column(window) else {
            return;
        };

        self.swap((column_idx, idx), space, direction);
    }

    pub fn swap(&mut self, (column_idx, idx): (usize, usize), space: &mut Space<Window>, direction: Direction) {

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
            let [col1, col2] = self
                .privileged
                .get_disjoint_mut([column_idx, other_column])
                .unwrap();

            let reference1 = space.element_location(&col1[0]).unwrap();
            let reference2 = space.element_location(&col2[0]).unwrap();
            let mut animation = self.animation.write().unwrap();

            for window in col1.iter() {
                let point = dbg!(space.element_location(window).unwrap());

                animation.schedule(Animation::Move(AnimationBase::<MoveAnimation>::new(
                    InfoType::Final(Point::new(reference2.x, point.y)),
                    window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::Linear,
                    0,
                )));
            }

            for window in col2.iter() {
                let point = dbg!(space.element_location(window).unwrap());

                animation.schedule(Animation::Move(AnimationBase::<MoveAnimation>::new(
                    InfoType::Final(Point::new(reference1.x, point.y)),
                    window.clone(),
                    space,
                    Duration::from_millis(150),
                    Easing::Linear,
                    0,
                )));
            }

            self.privileged.swap(column_idx, other_column);
        } else {
            // swap windows within column
            let column = &mut self.privileged[column_idx];
            let win1 = column.get(idx).unwrap();
            let win2 = column.get(other_idx).unwrap();

            let win1_pos = space.element_location(win1).unwrap();
            let win2_pos = space.element_location(win2).unwrap();

            let mut animation = self.animation.write().unwrap();

            animation.schedule(Animation::Move(AnimationBase::<MoveAnimation>::new(
                InfoType::Final(win2_pos),
                win1.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
                0,
            )));            

            animation.schedule(Animation::Move(AnimationBase::<MoveAnimation>::new(
                InfoType::Final(win1_pos),
                win2.clone(),
                space,
                Duration::from_millis(150),
                Easing::EaseInOut,
                0,
            )));

            column.swap(idx, other_idx);
        }
    }

    fn find_column(&self, window: &Window) -> Option<(usize, usize)> {
        self.privileged
            .iter()
            .enumerate()
            .filter_map(|(col_idx, col)| {
                col.iter()
                    .enumerate()
                    .find(|(_, x)| x.id() == window.id())
                    .map(|x| ((col_idx, col), x))
            })
            .next()
            .map(|((col_idx, _), (idx, _))| (col_idx, idx))
    }

    pub fn remove(&mut self, window: Window, space: &mut Space<Window>) {
        let Some((column_idx, idx)) = self.find_column(&window) else {
            return;
        };

        let col = &mut self.privileged[column_idx];
        let window = &col[idx];

        space.unmap_elem(window);
        let size = window.geometry().size;

        if col.len() > 1 {
            col.remove(idx);
            self.redo_height(space, size.h, column_idx);
        } else {
            self.privileged.remove(column_idx);
            self.redo_widths(space, size.w);
        }
    }

    /// NOT WORKING:
    /// window.geometry() doesn't work as i thought, need to find the window position
    /// by knowing the layout, which isn't completed yet, so
    /// TODO, FIXME
    pub fn find_window(&self, point: Point<i32, Logical>) -> Option<&Window> {
        for col in self.privileged.iter() {
            let tester = &col[0];
            let mut rect = tester.geometry();
            rect.size.h = self.area.size.h;

            // found the column
            if rect.contains(point) {
                for window in col {
                    if window.geometry().contains(point) {
                        return Some(window);
                    }
                }

                // haven't found it in the right column, it's in some weird gap between windows or something
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
        for (n, col) in self.privileged.iter().enumerate() {
            let tester = &col[0];
            let Some(mut rect) = space.element_geometry(tester) else {
                continue;
            };
            dbg!(rect);
            rect.size.h = self.area.size.h;

            dbg!(rect);
            // found the column
            if rect.contains(point) {
                println!("hit column: {}", n);
                for window in col {
                    if window.geometry().contains(point) {
                        let pos = space.element_location(window).unwrap();
                        return Some((window, pos));
                    }
                }

                // haven't found it in the right column, it's in some weird gap between windows or something
                break;
            }
        }

        None
    }
}
