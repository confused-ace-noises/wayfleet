use std::{mem, sync::{Arc, RwLock}, time::Duration};

use smithay::{desktop::{Space, Window}, utils::{Logical, Rectangle, Size}};

use crate::{animations::{Easing, InfoType, MoveAnimation}, layout::{CropRect, IntoWayfleetWindow, WayfleetWindow, controller::{LayoutController, ResizeType}}};

use super::{Map, coordinate::Coordinate, tile::{Tile, TileType}};

impl Map {
    pub fn new_soft_insert(&mut self, window: Window, space: &Space<WayfleetWindow>, crop_rect: CropRect) -> Option<WayfleetWindow> {
        let first_avail_coord = self.first_available?;
        
        let mut new_size = Size::new(self.cell_width, self.cell_height);
        
        if let Err(min_size) = LayoutController::validate_size(&window, new_size) {
            new_size = min_size;
            self.resize_all_cells(Some(min_size.w), Some(min_size.h), space);
        }
        
        LayoutController::resize(space, &window, ResizeType::Both(new_size));
        
        let pos = self.get_position_shifted(first_avail_coord);
        let wayfleet_window = window.as_map(crop_rect, Arc::new(RwLock::new(Rectangle { loc: pos, size: new_size })));

        self[&first_avail_coord] = Some(Tile::new_leader(wayfleet_window.clone(), first_avail_coord));

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
        
        self.new_focus_at(first_avail_coord, space);
        
        self.recalculate_available();

        Some(wayfleet_window)
        
    }

    pub fn new_insert(&mut self, window: Window, space: &mut Space<WayfleetWindow>, crop_rect: Arc<RwLock<Rectangle<i32, Logical>>>) -> Option<WayfleetWindow> {
        let first_avail_coord = self.first_available?;
        
        let mut new_size = Size::new(self.cell_width, self.cell_height);
        
        if let Err(min_size) = LayoutController::validate_size(&window, new_size) {
            new_size = min_size;
            self.resize_all_cells(Some(min_size.w), Some(min_size.h), space);
        }
        
        LayoutController::resize(space, &window, ResizeType::Both(new_size));
        
        let pos = self.get_position_shifted(first_avail_coord);
        let wayfleet_window = window.as_map(crop_rect, Arc::new(RwLock::new(Rectangle { loc: pos, size: new_size })));

        self[&first_avail_coord] = Some(Tile::new_leader(wayfleet_window.clone(), first_avail_coord));

        space.map_element(wayfleet_window.clone(), pos, true);
        self.new_focus_at(first_avail_coord, space);
        
        self.recalculate_available();

        Some(wayfleet_window)
    }

    #[deprecated]
    pub fn old_insert(&mut self, window: WayfleetWindow) -> Option<Coordinate> {
        if let Some(coord) = self.first_available {
            self[&coord] = Some(Tile::new_leader(window, coord));
            self.recalculate_available();
            Some(coord)
        } else {
            None
        }
    }

    #[deprecated]
    pub fn old_insert_at(&mut self, window: WayfleetWindow, position: &Coordinate) -> bool {
        let x = &mut self[position];

        if x.is_none() {
            *x = Some(Tile::new_leader(window, *position));
            if let Some(available) = self.first_available
                && *position == available
            {
                self.recalculate_available();
            }
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, position: &Coordinate, space: &mut Space<WayfleetWindow>) -> Option<Vec<Tile>> {
        if let Some(x) = self.focus && x == *position {
            self.focus = None
        } 

        let tile = self[position].as_ref()?;
        let Tile { tile_type: TileType::Leader { rows, cols, coord }, ref window }: Tile = *self.get_leader(tile) else { unreachable!() };

        space.unmap_elem(window);

        let mut vec = vec![];

        for r in 0..=rows {
            for c in 0..=cols {
                vec.push(mem::take(&mut self.map[coord.row as usize + r][coord.column as usize + c]))
            }
        }

        self.recalculate_available();

        if vec.iter().any(Option::is_none) {
            None
        } else {
            Some(vec.into_iter().flatten().collect())   
        }
    }

    pub fn soft_remove(&mut self, position: &Coordinate) -> Option<Vec<Tile>> {
        if let Some(x) = self.focus && x == *position {
            self.focus = None
        } 

        let tile = self[position].as_ref()?;
        let Tile { tile_type: TileType::Leader { rows, cols, coord }, .. }: Tile = *self.get_leader(tile) else { unreachable!() };

        let mut vec = vec![];

        for r in 0..=rows {
            for c in 0..=cols {
                vec.push(mem::take(&mut self.map[coord.row as usize + r][coord.column as usize + c]))
            }
        }

        self.recalculate_available();

        if vec.iter().any(Option::is_none) {
            None
        } else {
            Some(vec.into_iter().flatten().collect())   
        }
    }

    #[allow(unused)]
    fn remove_single(&mut self, position: &Coordinate) -> Option<Tile> {
        mem::take(&mut self[position])
    }
    
    pub fn recalculate_available(&mut self) {
        if let Some(avail @ Coordinate { row, column }) = self.first_available && self.is_valid_coord(avail) {
            let mut found = false;
            // first try behind
            'outer: for r in 0..=row {
                for c in 0..=column {
                    if self.map[r as usize][c as usize].is_none() {
                        self.first_available = Some(Coordinate { row: r, column: c });
                        found = true;
                        break 'outer;
                    }
                }
            }
            
            let mut mut_column = column; 
            // try in front
            if !found {
                'outer: for r in (row as usize)..self.rows {
                    for c in (mut_column as usize)..self.columns {
                        if self.map[r][c].is_none() {
                            self.first_available = Some(Coordinate {
                                row: r as i32,
                                column: c as i32,
                            });
                            found = true;
                            break 'outer;
                        }
                    }
                    mut_column = 0;
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
                        self.first_available = Some(Coordinate {
                            row: r as i32,
                            column: c as i32,
                        });
                        break 'outer;
                    }
                }
            }
        }
    }
}