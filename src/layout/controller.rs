use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use smithay::{
    desktop::{Space, Window},
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER, Size},
    wayland::seat::WaylandFocus,
};
use wayfleet_config::Config;

use crate::{
    animations::{AnimationController, AnimationHandle},
    layout::{
        map::{
            Map,
            coordinate::{Coordinate, Direction},
        },
        privileged::Privileged,
    },
    state::{OutputState, State},
};

#[derive(Debug, Clone)]
pub enum Focus {
    None,
    Map(Window),
    Privileged(Window),
}

#[derive(Debug)]
pub struct LayoutController {
    pub map: Map,
    pub privileged: Privileged,
    pub space: Space<Window>,
    pub animation: AnimationHandle,
    pub focus: Focus,
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
            map,
            privileged,
            space: Space::default(),
            animation,
            focus: Focus::None,
        }
    }

    pub fn insert_generic(&mut self, window: Window) -> InsertResult {
        if let Some(coord) = self.map.insert(window.clone()) {
            // TODO: move space mapping to insert
            let pos = self.map.get_position(coord);
            self.space.map_element(window.clone(), pos, true);
            self.space.refresh();
            self.map.new_focus(&window);
            self.focus = Focus::Map(window);
            InsertResult::InMap(coord)
        } else {
            let pos = self
                .privileged
                .insert_right_of_focus(window.clone(), &self.space)
                .unwrap_or_else(|| self.privileged.insert_new_last(window.clone(), &self.space));
            self.space.map_element(window.clone(), pos, true);
            self.space.refresh();
            self.privileged.new_focus(&window, &self.space);
            self.focus = Focus::Privileged(window);
            InsertResult::InPrivileged
        }
    }

    pub fn insert_by_focus(&mut self, window: Window) -> InsertResult {
        match &self.focus {
            Focus::Privileged(_) => {
                let pos = self
                    .privileged
                    .insert_right_of_focus(window.clone(), &self.space)
                    .unwrap_or_else(|| self.privileged.insert_new_last(window.clone(), &self.space));
                self.space.map_element(window.clone(), pos, true);
                self.space.refresh();
                self.privileged.new_focus(&window, &self.space);
                self.focus = Focus::Privileged(window);
                InsertResult::InPrivileged
            },
            _ => self.insert_generic(window),
        }
    }

    pub fn insert_priv(&mut self, window: Window) {
        let pos = self.privileged.insert_right_of_focus(window.clone(), &self.space).unwrap_or_else(|| self.privileged.insert_new_last(window.clone(), &self.space));
        self.space.map_element(window, pos, true);
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

    pub fn move_focus(state: &mut State, direction: Direction) {
        let _self = &mut state.layout;

        match _self.focus.clone() {
            Focus::Map(old) => {
                let x = _self.map.shift_focus(direction);

                match x {
                    super::map::focus::ShiftFocusOutput::Success(window) => {
                        state.refocus(&old, &window);
                        state.layout.focus = Focus::Map(window);
                    }
                    super::map::focus::ShiftFocusOutput::Invalid => {}
                    super::map::focus::ShiftFocusOutput::OutOfBounds => {}
                    super::map::focus::ShiftFocusOutput::OutOfBoundsHinted(hint) => {
                        if let Some(new) = _self.privileged.new_focus_hinted(hint, &_self.space) {
                            state.refocus(&old, &new);
                            state.layout.focus = Focus::Map(new);
                        }
                    }
                }
            }
            Focus::Privileged(old) => {
                let x = _self.privileged.shift_focus(direction, &_self.space);

                match x {
                    super::map::focus::ShiftFocusOutput::Success(window) => {
                        state.refocus(&old, &window);
                        state.layout.focus = Focus::Privileged(window);
                    }
                    super::map::focus::ShiftFocusOutput::Invalid => {}
                    super::map::focus::ShiftFocusOutput::OutOfBounds => {}
                    super::map::focus::ShiftFocusOutput::OutOfBoundsHinted(hint) => {
                        if let Some(new) = _self.map.new_focus_hinted(hint, &_self.space) {
                            state.refocus(&old, &new);
                            state.layout.focus = Focus::Privileged(new)
                        }
                    }
                }
            }
            Focus::None => {}
        }
    }

    pub fn new_focus(state: &mut State, window: Window) {
        let mut old_window = None;

        match &state.layout.focus {
            Focus::Map(window) => old_window = Some(window.clone()),
            Focus::Privileged(window) => old_window = Some(window.clone()),
            Focus::None => {}
        }

        if !state.layout.map.new_focus(&window) {
            state
                .layout
                .privileged
                .new_focus(&window, &state.layout.space);
            if let Some(old) = old_window {
                state.refocus(&old, &window);
            }
            state.layout.focus = Focus::Privileged(window);
        } else {
            if let Some(old) = old_window {
                state.refocus(&old, &window);
            }
            state.layout.focus = Focus::Map(window);
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
        if let Some(tile) = state.layout.map.search_tile(window) {
            if let Focus::Map(win) = &state.layout.focus
                && *win == *window
            {
                // needs to refocus onto something else somehow
                let old_win = win.clone();
                let window = state.layout.map.radial_search(tile).cloned();

                if let Some(window) = window {
                    // if the radial search found somehting, set that
                    state.refocus(&old_win, &window);
                    state.layout.map.new_focus(&window);
                    state.layout.focus = Focus::Map(window);
                } else if let Some(window) = state
                    .layout
                    .space
                    .elements()
                    .cloned()
                    .collect::<Vec<_>>()
                    .first()
                {
                    // if radial search didn't find anything, just get the first one
                    state.layout.map.focus = None;
                    let window = window.clone();
                    state
                        .layout
                        .privileged
                        .new_focus(&window, &state.layout.space);
                    state.refocus(&old_win, &window);
                    state.layout.focus = Focus::Privileged(window.clone());
                } else {
                    // if nothing is found at all, we just don't have a focus
                    state.defocus(&old_win);
                }
            }
            state.layout.map.remove(&tile, &mut state.layout.space);
        } else {
            if let Focus::Privileged(win) = &state.layout.focus
                && *win == *window
            {
                let old_win = win.clone();
                // need to refocus
                let window = state.layout.privileged.radial_search(&old_win).cloned();

                if let Some(window) = window {
                    // if the radial search found somehting, set that
                    
                    state.refocus(&old_win, &window);
                    assert!(state
                        .layout
                        .privileged
                        .new_focus(&window, &state.layout.space));
                    state.layout.focus = Focus::Privileged(window);

                } else if let Some(window) = state
                    .layout
                    .space
                    .elements()
                    .cloned()
                    .collect::<Vec<_>>()
                    .first()
                {
                    // if radial search didn't find anything, just get the first one
                    state.layout.privileged.focused = None;
                    let window = window.clone();
                    state.refocus(&old_win, &window);
                    state.layout.map.new_focus(&window);
                    state.layout.focus = Focus::Map(window.clone());
                } else {
                    // if nothing is found at all, we just don't have a focus
                    state.defocus(&old_win);
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
