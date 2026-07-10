use std::{
    cell::RefCell, collections::HashSet, sync::{Arc, RwLock}, time::{Duration, Instant},
};

use smithay::{
    desktop::{Space, Window}, utils::{Logical, Point, Rectangle, Scale, Size},
};
use wayfleet_config::Config;

use crate::{layout::{
    animation::{AnimationController, AnimationHandle}, map::{Coordinate, Map}, privileged::Privileged,
}, state::OutputState};

pub struct LayoutController {
    pub map: Map,
    pub privileged: Privileged,
    pub space: Space<Window>,
    pub animation: AnimationHandle,
}

impl LayoutController {
    pub fn _new(
        config: Config
    ) -> Self {
        let animation = AnimationHandle(Arc::new(RwLock::new(AnimationController::new(Duration::from_millis(16)))));

        todo!()

        // Self {
        //     map: Map::new(
        //         rows,
        //         columns,
        //         cell_height,
        //         cell_width,
        //         Point::new(0, area.size.h),
        //         animation.clone()
        //     ),
        //     privileged: Privileged::new(area, animation.clone()),
        //     space: Space::default(),
        //     animation
        // }
    }

    pub fn new(
        config: Config,
        output_state: &OutputState,
    ) -> Self {
        let animation = AnimationHandle(Arc::new(RwLock::new(AnimationController::new(Duration::from_millis(16)))));

        let rect = Rectangle::new(Point::new(0, 0), Size::new(output_state.size.to_logical(output_state.scale_factor).w, output_state.size.to_logical(output_state.scale_factor).h / 3));
        let privileged = Privileged::new(rect, animation.clone());

        let map = Map::new(config.map, animation.clone(), &output_state, Point::new(100, rect.size.h));

        Self { map, privileged, space: Space::default(), animation }
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

    pub fn insert_priv(&mut self, window: Window) {
        let pos = self.privileged.insert(window.clone(), &mut self.space);
        self.space.map_element(window, pos, true);
    }

    pub fn resize(window: &Window, resize: ResizeType) -> Option<()> {
        let xdg = window.toplevel().unwrap();
        let out = xdg.with_pending_state(|state| match resize {
            ResizeType::Both(size) => {
                dbg!(state.size);
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
        //     self.privileged.find_window_pos(point, &self.space)
        // } else {
        //     // not in privileged, look at map
        //     println!("non-priv");
        //     self.map.find_window_pos(point, &self.space)
        // }

        self.space.element_under(point)
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
