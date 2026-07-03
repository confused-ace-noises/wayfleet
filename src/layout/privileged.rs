use smithay::{desktop::{Space, Window}, utils::{Logical, Point, Rectangle}};

use crate::layout::controller::{LayoutController, ResizeType};


pub struct Privileged {
    pub area: Rectangle<i32, Logical>,
    pub privileged: Vec<Vec<Window>>,
}

impl Privileged {
    pub fn new(area: Rectangle<i32, Logical>) -> Self {
        Self {
            privileged: vec![],
            area,
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
                let xdg = window.toplevel().unwrap();

                xdg.with_pending_state(|x| {
                    if let Some(size) = x.size.as_mut() {
                        size.w += single_width_delta;
                    }
                });

                xdg.send_pending_configure();

                let mut pos = space
                    .element_location(window)
                    .expect("well this shouldn't happen");
                pos.x += single_width_delta * n_col as i32;
                space.relocate_element(window, pos);
            }
        }
    }

    pub fn redo_height(&self, space: &mut Space<Window>, gained_height: i32, column_idx: usize) {
        let column = &self.privileged[column_idx];

        if column.is_empty() {
            return;
        }

        let delta = gained_height / column.len() as i32;

        for (idx, window) in column.iter().enumerate() {
            let xdg = window.toplevel().unwrap();

            xdg.with_pending_state(|x| {
                if let Some(size) = x.size.as_mut() {
                    size.h += delta;
                }
            });

            xdg.send_pending_configure();

            let mut pos = space
                .element_location(window)
                .expect("well this shouldn't happen");
            pos.y += delta * idx as i32;
            space.relocate_element(window, pos);
        }
    }

    fn find_column(&self, window: Window) -> Option<(usize, usize)> {
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
        let Some((column_idx, idx)) = self.find_column(window) else {
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
    pub fn find_window_pos(&self, point: Point<i32, Logical>, space: &Space<Window>) -> Option<(&Window, Point<i32, Logical>)> {
        for (n, col) in self.privileged.iter().enumerate() {
            let tester = &col[0];
            let Some(mut rect) = space.element_geometry(tester) else { continue };
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
