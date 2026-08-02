use std::{
    borrow::Borrow, collections::VecDeque, ops::DerefMut, sync::{Arc, RwLock, RwLockWriteGuard}, time::Duration,
};

use smithay::{
    desktop::{Space, Window}, reexports::wayland_protocols::xdg::shell::server::xdg_toplevel, utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Size}, wayland::{compositor::with_states, seat::WaylandFocus, shell::xdg::SurfaceCachedState },
};
use wayfleet_config::{Config, amount::SetSizeAmount};

use crate::{
    animations::{AnimationController, AnimationHandle}, layout::{
        WayfleetWindow, map::{
            Map,
            coordinate::Direction,
        }, privileged::Privileged,
    }, state::{CONFIG, OutputState, State},
};

#[derive(Debug, Clone)]
pub enum Focus {
    None,
    Map(WayfleetWindow),
    Privileged(WayfleetWindow),
}

#[derive(Debug, Clone, Copy)]
pub enum ForceSpawn {
    Map, 
    Priv
}

#[derive(Debug)]
pub struct LayoutController {
    pub map: Map,
    pub privileged: Privileged,
    pub space: Space<WayfleetWindow>,
    pub animation: AnimationHandle,
    pub focus: Focus,
    pub map_clip: Arc<RwLock<Rectangle<i32, Logical>>>,
    pub output_state: OutputState,
    pub forced_windows: VecDeque<ForceSpawn>,
}

impl LayoutController {
    pub fn new(config: &Config, output_state: &OutputState) -> Self {
        // TODO: figure out animation tick frequency
        let animation = AnimationHandle(Arc::new(RwLock::new(AnimationController::new(
            Duration::from_millis(16),
        ))));

        let privileged =
            Privileged::new(&config.layout.privileged, output_state, animation.clone());

        let map = Map::new(
            &config.layout.map,
            animation.clone(),
            output_state,
            privileged.map_offset,
        );

        Self {
            map_clip: Arc::new(RwLock::new(map.viewport)),
            map,
            privileged,
            space: Space::default(),
            animation,
            focus: Focus::None,
            output_state: output_state.clone(),
            forced_windows: VecDeque::new(),
        }
    }

    pub fn set_unfocused_inner_window(&mut self) {
        match &mut self.focus {
            Focus::Map(wayfleet_window) => wayfleet_window.focused(false),
            Focus::Privileged(wayfleet_window) => wayfleet_window.focused(false),
            Focus::None => {},
        }
    }

    pub fn set_focused_inner_window(&mut self) {
        match &mut self.focus {
            Focus::Map(wayfleet_window) => wayfleet_window.focused(true),
            Focus::Privileged(wayfleet_window) => wayfleet_window.focused(true),
            Focus::None => {},
        }
    }

    pub fn insert_generic(&mut self, window: Window) -> InsertResult {
        if let Some(window) = self.map.new_insert(window.clone(), &mut self.space, self.map_clip.clone()) {
            self.space.refresh();
            self.set_unfocused_inner_window();
            self.focus = Focus::Map(window);
            self.set_focused_inner_window();
            InsertResult::InMap
        } else {
            self.insert_priv(window);
            InsertResult::InPrivileged
        }
    }

    pub fn insert_by_focus(&mut self, window: Window) -> InsertResult {
        match &self.focus {
            Focus::Privileged(_) => {
                self.insert_priv(window);
                InsertResult::InPrivileged
            },
            _ => self.insert_generic(window),
        }
    }
    
    pub fn insert_by_focus_w_forcing(&mut self, window: Window) -> InsertResult {
        if let Some(ty) = self.forced_windows.pop_front() {
            match ty {
                ForceSpawn::Priv => {
                    self.insert_priv(window);
                    InsertResult::InPrivileged
                },
                ForceSpawn::Map => self.insert_generic(window),
            }
        } else {
            self.insert_by_focus(window)
        }
    }

    pub fn insert_priv(&mut self, window: Window) {
        let (window, has_to_update_map) = self.privileged.new_insert_right_of_focus(window, &mut self.space);

        if let Some(update) = has_to_update_map {
            self.map.move_offset(Point::new(self.map.viewport.loc.x, update), &mut self.space);
            let mut lock = self.map_clip.write().unwrap();
            *lock = self.map.viewport;
        }

        self.space.refresh();
        self.set_unfocused_inner_window();
        self.focus = Focus::Privileged(window);
        self.set_focused_inner_window();
    }

    pub fn resize(window: &Window, resize: ResizeType) -> Option<()> {
        let xdg = window.toplevel().unwrap();
        let out = xdg.with_pending_state(|state| match resize {
            ResizeType::Both(size) => {
                // dbg!(state.size);
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

    pub fn resize_delta(window: &Window, resize: ResizeType) -> Option<()> {
        let xdg = window.toplevel().unwrap();
        let out = xdg.with_pending_state(|state| match resize {
            ResizeType::Both(size) => {
                state.size = Some(state.size.unwrap_or(Size::default()) + size);
                Some(())
            }
            ResizeType::Width(w) => {
                if let Some(size) = state.size {
                    let size = Size::new(size.w + w, size.h);
                    state.size = Some(size);
                    Some(())
                } else {
                    None
                }
            }
            ResizeType::Height(h) => {
                if let Some(size) = state.size {
                    let size = Size::new(size.w, size.h + h);
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

    pub fn tick_animation(&mut self) {
        let mut lock = self.animation.write().unwrap();
        lock.tick(&mut self.space);
    }

    /*
    // TODO: switch to faster algorithm once layout is fleshed out
    pub fn find_window(&self, point: Point<f64, Logical>) -> Option<&Window> {
        // * faster algo
        // if self.privileged.area.contains(point) {
        //     // it's in the privileged
        //     self.privileged.find_window(point)
        // } else {
        //     // not in privileged, look at map
        //     self.map.find_window(point)
        // }
        self.space.element_under(point).map(|x| x.0)
    }

    // TODO: switch to faster algorithm once layout is fleshed out
    pub fn find_window_pos(
        &self,
        point: Point<f64, Logical>,
    ) -> Option<(&Window, Point<i32, Logical>)> {
        // * faster algo
        // if self.privileged.area.contains(point) {
        //     // it's in the privileged
        //     println!("priv");
        //     self.privileged.find_window_pos(point, &self.space)S, o
        // } else {
        //     // not in privileged, look at map
        //     println!("non-priv");
        //     self.map.find_window_pos(point, &self.space)
        // }

        self.space.element_under(point)
    }
    */

    pub fn find_window(&self, point: Point<f64, Logical>) -> Option<&WayfleetWindow> {
        self.find_window_pos(point).map(|x| x.0)
    }

    pub fn find_window_pos(&self, point: Point<f64, Logical>) -> Option<(&WayfleetWindow, Point<i32, Logical>)> {
        self.space.element_under(point)
    }

    pub fn move_focus(state: &mut State, direction: Direction) {
        let _self = &mut state.layout;

        match _self.focus.clone() {
            Focus::Map(old) => {
                let x = _self.map.shift_focus(direction, &_self.space);

                match x {
                    super::map::focus::ShiftFocusOutput::Success(window) => {
                        state.refocus(&old, &window);
                        state.layout.set_unfocused_inner_window();
                        state.layout.focus = Focus::Map(window);
                        state.layout.set_focused_inner_window();
                    }
                    super::map::focus::ShiftFocusOutput::Invalid => {}
                    super::map::focus::ShiftFocusOutput::OutOfBounds => {}
                    super::map::focus::ShiftFocusOutput::OutOfBoundsHinted(hint) => {
                        if let Some(new) = _self.privileged.new_focus_hinted(hint, &_self.space) {
                            state.refocus(&old, &new);
                            state.layout.set_unfocused_inner_window();
                            state.layout.focus = Focus::Privileged(new);
                            state.layout.set_focused_inner_window();
                        }
                    }
                }
            }
            Focus::Privileged(old) => {
                let x = _self.privileged.shift_focus(direction, &_self.space);

                match x {
                    super::map::focus::ShiftFocusOutput::Success(window) => {
                        state.refocus(&old, &window);
                        state.layout.set_unfocused_inner_window();
                        state.layout.focus = Focus::Privileged(window);
                        state.layout.set_focused_inner_window();
                    }
                    super::map::focus::ShiftFocusOutput::Invalid => {}
                    super::map::focus::ShiftFocusOutput::OutOfBounds => {}
                    super::map::focus::ShiftFocusOutput::OutOfBoundsHinted(hint) => {
                        if let Some(new) = _self.map.new_focus_hinted(hint, &_self.space) {
                            state.refocus(&old, &new);
                            state.layout.set_unfocused_inner_window();
                            state.layout.focus = Focus::Map(new);
                            state.layout.set_focused_inner_window();
                        }
                    }
                }
            }
            Focus::None => {}
        }
    }

    pub fn new_focus(state: &mut State, window: WayfleetWindow) {
        let old_window = match &state.layout.focus {
            Focus::Map(window) =>  Some(window.clone()),
            Focus::Privileged(window) =>  Some(window.clone()),
            Focus::None => None
        };

        if state.layout.map.new_focus(&window, &state.layout.space) {
            if let Some(old) = old_window {
                state.refocus(&old, &window);
            }
            state.layout.set_unfocused_inner_window();
            state.layout.focus = Focus::Map(window);
            state.layout.set_focused_inner_window();
        } else {
            state
                .layout
                .privileged
                .new_focus(&window, &state.layout.space);
            if let Some(old) = old_window {
                state.refocus(&old, &window);
            }
            state.layout.set_unfocused_inner_window();
            state.layout.focus = Focus::Privileged(window);
            state.layout.set_focused_inner_window();
        }
    }

    pub fn currently_focused(&self) -> Option<&Window> {
        match &self.focus {
            Focus::None => None,
            Focus::Map(window) => Some(window),
            Focus::Privileged(window) => Some(window),
        }
    }

    pub fn swap_focused(&mut self, direction: Direction) {
        match &self.focus {
            Focus::Map(_) => {
                self.map.swap_or_move_focused(direction, &mut self.space);
            },
            Focus::Privileged(_) => {
                self.privileged.swap_focused(direction, &mut self.space);
            },
            Focus::None => {},
        }
    }

    pub fn remove(state: &mut State, window: &Window) {
        if let Some(tile) = state.layout.map.search_tile_window(window) {
            if let Focus::Map(win) = &state.layout.focus
                && *win == *window
            {
                // needs to refocus onto something else somehow
                let old_win = win.clone();
                let new_focus = state.layout.map.radial_search(tile).cloned();

                if let Some(window) = new_focus {
                    // if the radial search found somehting, set that
                    state.refocus(&old_win, &window);
                    state.layout.map.new_focus(&window, &state.layout.space);
                    state.layout.set_unfocused_inner_window();
                    state.layout.focus = Focus::Map(window);
                    state.layout.set_focused_inner_window();
                } else if let Some(window) = state
                    .layout
                    .space
                    .elements()
                    .filter(|&x| x != window)
                    .cloned()
                    .collect::<Vec<_>>()
                    .first()
                {
                    // if radial search didn't find anything, just get the first one
                    state.layout.map.viewport_offset = Point::<i32, Logical>::new(0, 0);
                    state.layout.map.focus = None;
                    let window = window.clone();
                    state
                        .layout
                        .privileged
                        .new_focus(&window, &state.layout.space);
                    state.refocus(&old_win, &window);
                    state.layout.set_unfocused_inner_window();
                    state.layout.focus = Focus::Privileged(window.clone());
                    state.layout.set_focused_inner_window();
                } else {
                    // if nothing is found at all, we just don't have a focus
                    state.defocus(&old_win);
                    state.layout.set_unfocused_inner_window();
                    state.layout.focus = Focus::None;
                }
            }
            state.layout.map.remove(&tile, &mut state.layout.space);
        } else {
            if let Focus::Privileged(win) = &state.layout.focus
                && *win == *window
            {
                let old_win = win.clone();
                // need to refocus
                let new_focus = state.layout.privileged.radial_search(&old_win).cloned();

                if let Some(window) = new_focus {
                    // if the radial search found somehting, set that
                    
                    state.refocus(&old_win, &window);
                    assert!(state
                        .layout
                        .privileged
                        .new_focus(&window, &state.layout.space));
                    state.layout.set_unfocused_inner_window();
                    state.layout.focus = Focus::Privileged(window);
                    state.layout.set_focused_inner_window();

                } else if let Some(window) = state
                    .layout
                    .space
                    .elements()
                    .filter(|&x| x != window)
                    .cloned()
                    .collect::<Vec<_>>()
                    .first()
                {
                    // if radial search didn't find anything, just get the first one
                    state.layout.privileged.focused = None;
                    state.layout.privileged.right_shift = 0;
                    let window = window.clone();
                    state.refocus(&old_win, &window);
                    state.layout.map.new_focus(&window, &state.layout.space);
                    state.layout.set_unfocused_inner_window();
                    state.layout.focus = Focus::Map(window.clone());
                    state.layout.set_focused_inner_window();
                } else {
                    // if nothing is found at all, we just don't have a focus
                    state.defocus(&old_win);
                    state.layout.set_unfocused_inner_window();
                    state.layout.focus = Focus::None;
                    state.layout.set_focused_inner_window();
                }
            }
            // let window = state.layout.privileged.radial_search(window, &state.layout.space);
            state
                .layout
                .privileged
                .remove(window.clone(), &mut state.layout.space);
        }
    }

    pub fn push_privileged_laterally(&mut self, direction: Direction) {
        if let Focus::Privileged(_) = &self.focus {
            self.privileged
                .push_focus_laterally(direction, &mut self.space);
        }
    }

    pub fn resize_cells_map(&mut self, resize: SetSizeAmount, is_height: bool) {
        let cell_amount   = if is_height {   self.map.cell_height   } else {   self.map.cell_width    };
        let screen_amount = if is_height { self.output_state.size.h } else { self.output_state.size.w };

        let final_resize = resize.get_final_resize(cell_amount, screen_amount);

        if final_resize <= 0 { return; }
        
        if is_height {
            self.map.resize_all_cells(None, Some(final_resize), &self.space);
        } else {
            self.map.resize_all_cells(Some(final_resize), None, &self.space);
        }
    }

    pub fn resize_column_height_privileged(&mut self, resize: SetSizeAmount) {
        let screen_amount = self.output_state.size.h;
        
        let config = CONFIG.get().unwrap();

        let final_resize = resize.get_delta_resize(self.privileged.std_size.h, screen_amount);
        let map_offset = self.privileged.set_height_all_cells_delta(dbg!(final_resize), &self.space);
        self.map.move_offset_delta(Point::new(0, map_offset + config.layout.privileged.padding.down + config.layout.map.padding.top - self.map.viewport.loc.y), &mut self.space);
        let mut lock = self.map_clip.write().unwrap();
        *lock = self.map.viewport; 
    }

    pub fn resize_cells_focused(&mut self, resize: SetSizeAmount, is_height: bool) {
        match self.focus {
            Focus::Map(_) => {
                self.resize_cells_map(resize, is_height);
            },
            Focus::Privileged(_) if is_height => {
                self.resize_column_height_privileged(resize);
            },
            _ => {},
        }
    }

    pub(super) fn validate_size(window: impl Borrow<Window>, new_size: Size<i32, Logical>) -> Result<(), Size<i32, Logical>> {
        let surface = window.borrow().wl_surface().unwrap();
        
        let min_size = with_states(&surface, |states| {
            let mut binding = states.cached_state.get::<SurfaceCachedState>();
            let data = binding.current();
            data.min_size
        });

        if min_size.h <= new_size.h && min_size.w <= new_size.w {
            Ok(())
        } else {
            Err(min_size)
        }
    }

    pub fn change_cell_size_map_if_focused(&mut self, direction: Direction, remove: bool) -> Option<bool> {
        let Focus::Map(_) = self.focus else { return None; };
        self.map.change_cells_focused(direction, remove, &mut self.space)
    }

    pub fn move_focused_to_other_area(&mut self) {
        match &self.focus {
            Focus::Map(window) => {
                let coord = self.map.search_tile(window).unwrap();
                // even if this leaves an invalid focus coord on the map,
                // it should be fine anyways because it will be set again once
                // map regains focus
                self.map.soft_remove(&coord);
                self.privileged.new_insert_soft_last(window.window.clone(), &mut self.space);
                self.focus = Focus::Privileged(window.clone())
            },
            Focus::Privileged(window) => {
                // same as aboveì
                self.privileged.soft_remove(window.window.clone(), &self.space);
                self.map.new_soft_insert(window.window.clone(), &self.space, self.map_clip.clone());
                self.focus = Focus::Map(window.clone())
            },
            Focus::None => {},
        }
    }
    
    pub fn resize_focused_window_privileged(&mut self, amount: SetSizeAmount, column: bool) {
        let Some(focused) = self.privileged.focused else { return; };
        let tile = &self.privileged.privileged[focused.0][focused.1];
        let (current_size, output_size) = if column { (tile.size.w, self.output_state.size.w) } else { (tile.size.h, self.output_state.size.h) };

        let delta = amount.get_delta_resize(current_size, output_size);

        if column {
            self.privileged.resize_column(focused.0, delta, &self.space);
        } else {
            self.privileged.resize_window(focused, delta, &self.space);
        }
    }

    // TODO figure out this mess
    pub fn update_available_state(&mut self, available: Rectangle<i32, Logical>) {
        let Config {
            layout: wayfleet_config::Layout {
                map: wayfleet_config::Map { padding: padding_map, .. },
                privileged: wayfleet_config::Privileged { padding: padding_priv, .. },
                ..
            },
            ..
        } = CONFIG.get().unwrap().as_ref();

        let left = padding_map.left.max(padding_priv.left);
        let right = padding_map.right.max(padding_priv.right);
        let hor = left + right;

        let privileged_available = Rectangle::new(
            Point::new(available.loc.x + left, available.loc.y + padding_priv.top),
            Size::new(
                (available.size.w - hor).max(0),
                (available.size.h - padding_priv.top).max(0),
            ),
        );

        let mut binding = self.privileged.viewport.write().unwrap();
        let x = RwLockWriteGuard::deref_mut(&mut binding);
        *x = x.intersection(privileged_available).unwrap_or(privileged_available);
        let privileged_rect = *x;
        drop(binding);

        self.privileged.set_height_all_cells(privileged_rect.size.h, &self.space);

        let privileged_bottom = privileged_rect.loc.y + privileged_rect.size.h;
        let map_top = privileged_bottom + padding_priv.down + padding_map.top;
        let map_available = Rectangle::new(
            Point::new(available.loc.x + left, map_top),
            Size::new(
                (available.size.w - hor).max(0),
                (available.loc.y + available.size.h - map_top - padding_map.down).max(0),
            ),
        );

        self.map.viewport = self.map.viewport.intersection(map_available).unwrap_or(map_available);
        self.map.move_offset(self.map.viewport.loc, &mut self.space);
    }
}

impl State {
    pub fn defocus(&mut self, old: &Window) {
        if let Some(xdg) = old.toplevel() {
            xdg.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Activated);
            });

            xdg.send_pending_configure();
        }
    }

    pub fn refocus(&mut self, old: &Window, new: &Window) {
        if let Some(xdg) = old.toplevel() {
            xdg.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Activated);
            });

            xdg.send_pending_configure();
        }

        old.set_activated(false);

        let new_surface = new.wl_surface().map(|x| x.as_ref().clone());
        self.seat.get_keyboard().unwrap().set_focus(
            self,
            new_surface,
            SERIAL_COUNTER.next_serial(),
        );

        if let Some(xdg) = new.toplevel() {
            xdg.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Activated);
            });

            xdg.send_pending_configure();
        }
    }
}

pub enum ResizeType {
    Both(Size<i32, Logical>),
    Width(i32),
    Height(i32),
}

pub enum InsertResult {
    InMap,
    InPrivileged,
}

pub struct LayoutSettings {
    pub rows: usize,
    pub columns: usize,
    pub cell_height: i32,
    pub cell_width: i32,
    pub area: Rectangle<i32, Logical>,
}
