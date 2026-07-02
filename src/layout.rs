use std::{
    mem,
    ops::{Index, IndexMut},
};

use smithay::{
    desktop::{Space, Window},
    reexports::{
        reis::request::RequestError::TooManyTouches,
        wayland_protocols::xdg::shell::client::xdg_toplevel,
    },
    utils::{Logical, Point, Rectangle, Size},
    wayland::seat::WaylandFocus,
};

// TODO: add multi-cell windows, probably
pub struct Map {
    pub map: Vec<Vec<Option<Window>>>,
    pub first_available: Option<Coordinate>,
    pub rows: usize,
    pub columns: usize,
    pub cell_height: i32,
    pub cell_width: i32,
    pub offset: Point<i32, Logical>,
}

impl Map {
    pub fn new(
        rows: usize,
        columns: usize,
        cell_height: i32,
        cell_width: i32,
        offset: Point<i32, Logical>,
    ) -> Self {
        assert_ne!(rows, 0);
        assert_ne!(columns, 0);
        assert!(cell_height > 0);
        assert!(cell_width > 0);

        Self {
            map: vec![vec![None; columns]; rows],
            first_available: Some([0, 0].into()),
            rows,
            columns,
            cell_height,
            cell_width,
            offset,
        }
    }

    pub fn insert(&mut self, window: Window) -> Option<Coordinate> {
        if let Some(coord) = self.first_available {
            self[&coord] = Some(window);
            self.recalculate_available();
            Some(coord)
        } else {
            None
        }
    }

    pub fn insert_at(&mut self, window: Window, position: &Coordinate) -> bool {
        let x = &mut self[position];

        if x.is_none() {
            *x = Some(window);
            if let Some(available) = self.first_available && *position == available {
                self.recalculate_available();
            } 
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, position: &Coordinate) -> Option<Window> {
        mem::take(&mut self[position])
    }

    pub fn recalculate_available(&mut self) {
        if let Some(x@ Coordinate { row, mut column }) = self.first_available {
            let mut found = false;
            dbg!(x);
            // first try, in front
            'outer: for r in (row as usize)..self.rows {
                for c in (column as usize)..self.columns {
                    if self.map[r][c].is_none() {
                        self.first_available = Some(Coordinate { row: r as i32, column: c as i32});
                        found = true;
                        break 'outer;
                    }
                }
                column = 0;
            }

            dbg!(found);

            // try behind
            if !found {
                'outer: for r in 0..=row {
                    for c in 0..=column {
                        if self.map[r as usize][c as usize].is_none() {
                            self.first_available = Some(Coordinate { row: r, column: c });
                            found = true;
                            break 'outer;
                        }
                    }
                }
            }

            // still hasn't been found, all places are full
            if !found {
                self.first_available = None
            }
        } else {
            'outer: for r in 0..self.rows {
                for c in 0..self.columns {
                    if self.map[r][c].is_none() {
                        self.first_available = Some(Coordinate { row: r as i32, column: c as i32});
                        break 'outer;
                    }
                }
            }
        }

        dbg!(self.first_available);
    }

    pub fn get_position(&self, Coordinate { row, column }: Coordinate) -> Point<i32, Logical> {
        Point::new(
            column * self.cell_width + self.offset.x ,
            row * self.cell_height + self.offset.y ,
        )
    }

    pub fn get_size(&self) -> Size<i32, Logical> {
        Size::new(self.cell_width, self.cell_width)
    }
}

impl Index<&Coordinate> for Map {
    type Output = Option<Window>;

    fn index(&self, Coordinate { row, column }: &Coordinate) -> &Self::Output {
        &self.map[*row as usize][*column as usize]
    }
}

impl IndexMut<&Coordinate> for Map {
    fn index_mut(&mut self, Coordinate { row, column }: &Coordinate) -> &mut Self::Output {
        &mut self.map[*row as usize][*column as usize]
    }
}

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

        Point::new(self.area.loc.x + self.area.size.w - size.w, self.area.loc.y)
    }

    pub fn redo_widths(&self, space: &mut Space<Window>, removed_width: i32) {
        let columns = self.privileged.len() as i32;

        if columns == 0 {
            return;
        }

        let single_width_delta = removed_width / columns;

        for (n_col, column) in self.privileged.iter().enumerate() {
            for window in column {
                let xdg = window.toplevel().unwrap();
                
                xdg.with_pending_state(|x| {
                    if let Some(size) = x.size.as_mut() {
                        size.w += single_width_delta;
                    }
                });
                
                xdg.send_pending_configure();

                let mut pos = space.element_location(window).expect("well this shouldn't happen");
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

            let mut pos = space.element_location(window).expect("well this shouldn't happen");
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
        let size = window.bbox().size;
        
        if col.len() > 1 {
            col.remove(idx);
            self.redo_height(space, size.h, column_idx);
        } else {
            self.privileged.remove(column_idx);
            self.redo_widths(space, size.w);
        }

    }
}

pub struct LayoutController {
    pub map: Map,
    pub privileged: Privileged,
    pub space: Space<Window>,
}

impl LayoutController {
    pub fn _new(
        rows: usize,
        columns: usize,
        cell_height: i32,
        cell_width: i32,
        area: Rectangle<i32, Logical>,
    ) -> Self {
        Self {
            map: Map::new(rows, columns, cell_height, cell_width, Point::new(0, area.size.h)),
            privileged: Privileged::new(area),
            space: Space::default(),
        }
    }

    pub fn new(
        LayoutSettings {
            rows,
            columns,
            cell_height,
            cell_width,
            area,
        }: LayoutSettings,
    ) -> Self {
        Self::_new(rows, columns, cell_height, cell_width, area)
    }

    pub fn insert_generic(&mut self, window: Window) -> InsertResult {
        if let Some(coord) = self.map.insert(window.clone()) {
            let pos = self.map.get_position(coord);
            self.space.map_element(window, pos, true);
            InsertResult::InMap(coord)
        } else {
            let pos = self.privileged.insert(window.clone(), &mut self.space);
            self.space.map_element(window, pos, true);
            InsertResult::InPrivileged
        }
    }

    pub fn resize(window: &Window, resize: ResizeType) -> Option<()> {
        let xdg = window.toplevel().unwrap();
        let out = xdg.with_pending_state(|state| match resize {
            ResizeType::Both(size) => {
                state.size = Some(size);
                Some(())
            }
            ResizeType::Width(w) => {
                if let Some(size) = state.size {
                    let size = Size::new(w, size.h);
                    state.size = Some(size);
                    Some(())
                } else {
                    None
                }
            }
            ResizeType::Height(h) => {
                if let Some(size) = state.size {
                    let size = Size::new(size.w, h);
                    state.size = Some(size);
                    Some(())
                } else {
                    None
                }
            }
        });
        
        if out.is_some() {
            xdg.send_configure();
        }

        out
    }
}

pub enum ResizeType {
    Both(Size<i32, Logical>),
    Width(i32),
    Height(i32),
}

pub enum InsertResult {
    InMap(Coordinate),
    InPrivileged,
}

pub struct LayoutSettings {
    pub rows: usize,
    pub columns: usize,
    pub cell_height: i32,
    pub cell_width: i32,
    pub area: Rectangle<i32, Logical>,
}

#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd)]
pub struct Coordinate {
    pub row: i32,
    pub column: i32,
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
