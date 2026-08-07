use std::{
    borrow::Borrow, collections::VecDeque, ops::DerefMut, sync::{Arc, RwLock, RwLockWriteGuard}, time::Duration,
};

use smithay::{
    desktop::{LayerSurface, Space, Window, layer_map_for_output}, utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Size}, wayland::{compositor::with_states, seat::WaylandFocus, shell::xdg::SurfaceCachedState },
};
use wayfleet_config::{Config, amount::SetSizeAmount};

use crate::{
    animations::{AnimationController, AnimationHandle}, layout::{
        WayfleetWindow, map::{
            Map, MapHeight, coordinate::{Coordinate, Direction},
        }, privileged::{Height, Privileged},
    }, state::{BackendData, CONFIG, OutputState, State},
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
    pub is_layer_focused: bool,
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
            is_layer_focused: false,
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

    pub fn resize(space: &Space<WayfleetWindow>, window: &Window, resize: ResizeType) -> Option<()> {
        match window.underlying_surface() {
            smithay::desktop::WindowSurface::Wayland(toplevel_surface) => {
                let out = toplevel_surface.with_pending_state(|state| match resize {
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
                    toplevel_surface.send_configure();
                }

                out        
            },
            smithay::desktop::WindowSurface::X11(x11) => {
               let current = space.element_geometry(&WayfleetWindow::dummy(window.clone()))?; // TODO: check if this works

                match resize {
                    ResizeType::Both(size) => x11.configure(Rectangle::new(current.loc, size)).ok(),
                    ResizeType::Width(w) => x11.configure(Rectangle::new(current.loc, Size::new(w, current.size.h))).ok(),
                    ResizeType::Height(h) => x11.configure(Rectangle::new(current.loc, Size::new(current.size.w, h))).ok(),
                } 
            },
        }
    }


    // this method isn't used so im just commenting it out because
    // im too lazy to reimplement it for xwayalnd 
    /*
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
    */

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

    pub fn move_focus<BD: BackendData>(state: &mut State<BD>, direction: Direction) {
        let _self = state.backend_data.layout_controller_mut();

        match _self.focus.clone() {
            Focus::Map(old) => {
                let x = _self.map.shift_focus(direction, &_self.space);

                match x {
                    super::map::focus::ShiftFocusOutput::Success(window) => {
                        state.refocus(&old, &window);
                        
                        let _self = state.backend_data.layout_controller_mut();
                        _self.set_unfocused_inner_window();
                        _self.focus = Focus::Map(window);
                        _self.set_focused_inner_window();
                    }
                    super::map::focus::ShiftFocusOutput::Invalid => {}
                    super::map::focus::ShiftFocusOutput::OutOfBounds => {}
                    super::map::focus::ShiftFocusOutput::OutOfBoundsHinted(hint) => {
                        if let Some(new) = _self.privileged.new_focus_hinted(hint, &_self.space) {
                            state.refocus(&old, &new);
                                                        
                            let _self = state.backend_data.layout_controller_mut();
                            _self.set_unfocused_inner_window();
                            _self.focus = Focus::Privileged(new);
                            _self.set_focused_inner_window();
                        }
                    }
                }
            }
            Focus::Privileged(old) => {
                let x = _self.privileged.shift_focus(direction, &_self.space);

                match x {
                    super::map::focus::ShiftFocusOutput::Success(window) => {
                        state.refocus(&old, &window);
                        
                        let _self = state.backend_data.layout_controller_mut();
                        _self.set_unfocused_inner_window();
                        _self.focus = Focus::Privileged(window);
                        _self.set_focused_inner_window();
                    }
                    super::map::focus::ShiftFocusOutput::Invalid => {}
                    super::map::focus::ShiftFocusOutput::OutOfBounds => {}
                    super::map::focus::ShiftFocusOutput::OutOfBoundsHinted(hint) => {
                        if let Some(new) = _self.map.new_focus_hinted(hint, &_self.space) {
                            state.refocus(&old, &new);

                            let _self = state.backend_data.layout_controller_mut();
                            _self.set_unfocused_inner_window();
                            _self.focus = Focus::Map(new);
                            _self.set_focused_inner_window();
                        }
                    }
                }
            }
            Focus::None => {}
        }
    }

    pub fn new_focus<BD: BackendData>(state: &mut State<BD>, window: WayfleetWindow) {
        let _self = state.backend_data.layout_controller_mut();

        let old_window = match &_self.focus {
            Focus::Map(window) =>  Some(window.clone()),
            Focus::Privileged(window) =>  Some(window.clone()),
            Focus::None => None
        };

        if _self.map.new_focus(&window, &_self.space) {
            if let Some(old) = old_window {
                state.refocus(&old, &window);
            }

            let _self = state.backend_data.layout_controller_mut();
            _self.set_unfocused_inner_window();
            _self.focus = Focus::Map(window);
            _self.set_focused_inner_window();
        } else {
            _self
                .privileged
                .new_focus(&window, &_self.space);
            if let Some(old) = old_window {
                state.refocus(&old, &window);
            }
            
            let _self = state.backend_data.layout_controller_mut();
            _self.set_unfocused_inner_window();
            _self.focus = Focus::Privileged(window);
            _self.set_focused_inner_window();
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

    pub fn remove<BD: BackendData>(state: &mut State<BD>, window: &Window) {
        let _self = state.backend_data.layout_controller_mut();

        if let Some(tile) = _self.map.search_tile_window(window) {
            if let Focus::Map(win) = &_self.focus
                && *win == *window
            {
                // needs to refocus onto something else somehow
                let old_win = win.clone();
                let new_focus = _self.map.radial_search(tile).cloned();

                if let Some(window) = new_focus {
                    // if the radial search found somehting, set that
                    _self.map.new_focus(&window, &_self.space);
                    state.refocus(&old_win, &window);
                    
                    let _self = state.backend_data.layout_controller_mut();
                    _self.set_unfocused_inner_window();
                    _self.focus = Focus::Map(window);
                    _self.set_focused_inner_window();
                } else if let Some(window) = _self
                    .space
                    .elements()
                    .filter(|&x| x != window)
                    .cloned()
                    .collect::<Vec<_>>()
                    .first()
                {
                    // if radial search didn't find anything, just get the first one
                    _self.map.viewport_offset = Point::<i32, Logical>::new(0, 0);
                    _self.map.focus = None;
                    let window = window.clone();
                    _self
                        .privileged
                        .new_focus(&window, &_self.space);

                    state.refocus(&old_win, &window);
                    let _self = state.backend_data.layout_controller_mut();
                    _self.set_unfocused_inner_window();
                    _self.focus = Focus::Privileged(window.clone());
                    _self.set_focused_inner_window();
                } else {
                    // if nothing is found at all, we just don't have a focus
                    state.defocus(&old_win);

                    let _self = state.backend_data.layout_controller_mut();
                    _self.set_unfocused_inner_window();
                    _self.focus = Focus::None;
                }
            }

            let _self = state.backend_data.layout_controller_mut();

            _self.map.remove(&tile, &mut _self.space);
        } else {
            if let Focus::Privileged(win) = &_self.focus
                && *win == *window
            {
                let old_win = win.clone();
                // need to refocus
                let new_focus = _self.privileged.radial_search(&old_win).cloned();

                if let Some(window) = new_focus {
                    // if the radial search found somehting, set that
                    
                    _self
                        .privileged
                        .new_focus(&window, &_self.space);
                    state.refocus(&old_win, &window);

                    let _self = state.backend_data.layout_controller_mut();
                    _self.set_unfocused_inner_window();
                    _self.focus = Focus::Privileged(window);
                    _self.set_focused_inner_window();

                } else if let Some(window) = _self
                    .space
                    .elements()
                    .filter(|&x| x != window)
                    .cloned()
                    .collect::<Vec<_>>()
                    .first()
                {
                    // if radial search didn't find anything, just get the first one
                    _self.privileged.focused = None;
                    _self.privileged.right_shift = 0;
                    let window = window.clone();
                    _self.map.new_focus(&window, &_self.space);
                    state.refocus(&old_win, &window);

                    let _self = state.backend_data.layout_controller_mut();
                    _self.set_unfocused_inner_window();
                    _self.focus = Focus::Map(window.clone());
                    _self.set_focused_inner_window();
                } else {
                    // if nothing is found at all, we just don't have a focus
                    state.defocus(&old_win);

                    let _self = state.backend_data.layout_controller_mut();
                    _self.privileged.focused = None;
                    _self.set_unfocused_inner_window();
                    _self.focus = Focus::None;
                }
            }
            let _self = state.backend_data.layout_controller_mut();

            // let window = state.layout.privileged.radial_search(window, &state.layout.space);
            
            _self
                .privileged
                .remove(window.clone(), &mut _self.space);
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
        let Some(surface) = window.borrow().wl_surface() else  { return Ok(()) };
        
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

        let map_top = self.privileged.set_height_all_cells(privileged_rect.size.h, &self.space);

        let map_top = map_top + padding_map.top;
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

impl<BD: BackendData> State<BD> {
    pub fn resize_output(&mut self, output_state: OutputState) {
        
        let config = self.config.clone();
        let layout = self.backend_data.layout_controller();
        
        let mut lock = layout.privileged.viewport.write().unwrap();
        
        *lock = Privileged::output_state_to_rect(&output_state, &config.layout.privileged.padding, Height::FromOld((self.backend_data.output_state().clone(), *lock)));
        
        drop(lock);

        let mut lock = layout.map_clip.write().unwrap();
        
        let map_rect = Map::output_state_to_rect(&output_state, &config.layout.map.padding, MapHeight::FromOld((self.backend_data.output_state().clone(), layout.map.viewport)));

        *lock = map_rect;

        drop(lock);

        let layout = self.layout_mut();
        layout.map.viewport = map_rect;

        if let Some(mut layer_map) = layout.space.outputs().map(|x| layer_map_for_output(x)).next() {
            layer_map.arrange();
        }
        
    }

    pub fn focus_layer(&mut self, layer: &LayerSurface) {
        if layer.can_receive_keyboard_focus() {
            let _self = self.backend_data.layout_controller_mut();
        
            _self.is_layer_focused = true;

            match _self.focus.clone() {
                Focus::None => {},
                Focus::Map(wayfleet_window) => self.defocus(&wayfleet_window),
                Focus::Privileged(wayfleet_window) => self.defocus(&wayfleet_window),
            }
            
            let _self = self.backend_data.layout_controller_mut();

            _self.set_unfocused_inner_window();
                
            _self.focus = Focus::None;

            self.seat.get_keyboard().unwrap().set_focus(
                self,
                Some(layer.wl_surface().clone()),
                SERIAL_COUNTER.next_serial(),
            );
        }
    }

    pub fn defocus_layer(&mut self) {
        let layout = self.backend_data.layout_controller_mut();
        if layout.is_layer_focused {
            layout.is_layer_focused = false;

            let maybe_window = layout.map.radial_search(Coordinate { row : 0, column: 0 }).cloned();

            if let Some(window) = maybe_window {
                layout.map.new_focus(&window, &layout.space);
                layout.focus = Focus::Map(window.clone());
                layout.set_focused_inner_window();
              
                self.seat.get_keyboard().unwrap().set_focus(
                    self,
                    Some(window.wl_surface().unwrap().into_owned().clone()),
                    SERIAL_COUNTER.next_serial(),
                );
            } else if let Some(first) = layout
                .space
                .elements()
                .cloned()
                .collect::<Vec<_>>()
                .first()
            {
                layout.privileged.new_focus(first, &layout.space);
                layout.focus = Focus::Privileged(first.clone());
                layout.set_focused_inner_window();

                self.seat.get_keyboard().unwrap().set_focus(
                    self,
                    Some(first.wl_surface().unwrap().into_owned().clone()),
                    SERIAL_COUNTER.next_serial(),
                );
            } else {
                self.seat.get_keyboard().unwrap().set_focus(
                    self,
                    None,
                    SERIAL_COUNTER.next_serial(),
                );
            }
        }
    }

    pub fn defocus(&mut self, old: &Window) {
        // match old.underlying_surface() {
        //     smithay::desktop::WindowSurface::Wayland(toplevel) => {            
        //         toplevel.with_pending_state(|state| {
        //             state.states.unset(xdg_toplevel::State::Activated);
        //         });
    
        //         toplevel.send_pending_configure();
        //     },
        //     smithay::desktop::WindowSurface::X11(x11_surface) => {
        //         x11_surface.set_activated(false).unwrap();
        //     },
        // }

        old.set_activated(false);
    }

    pub fn refocus(&mut self, old: &Window, new: &Window) {
        
        old.set_activated(false);

        if let Some(xdg) = old.toplevel() {
            xdg.send_pending_configure();
        }

        let new_surface = new.wl_surface().map(|x| x.as_ref().clone());
        self.seat.get_keyboard().unwrap().set_focus(
            self,
            new_surface,
            SERIAL_COUNTER.next_serial(),
        );
        
        new.set_activated(true);

        if let Some(xdg) = new.toplevel() {
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
