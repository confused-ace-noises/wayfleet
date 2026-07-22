use std::fmt;

use smithay::{desktop::{Space, Window}, utils::{Logical, Point, Size}};

use crate::layout::controller::{LayoutController, ResizeType};

use super::InfoType;

pub trait AnimationDriver: fmt::Debug + Send + Sync {
    type Value: Lerp + Into<(i32, i32)> + fmt::Debug + Clone + Send + Sync + 'static;
    fn drive(&mut self, value: &Self::Value, window: &Window, space: &mut Space<Window>);
    fn start_end(&self, info: InfoType<Self::Value>, window: &Window, space: &Space<Window>) -> (Self::Value, Self::Value);
    fn init() -> Self;
}

#[derive(Debug)]
pub struct MoveAnimation;

impl AnimationDriver for MoveAnimation {
    type Value = Point<i32, Logical>;

    fn drive(&mut self, value: &Self::Value, window: &Window, space: &mut Space<Window>) {
        space.relocate_element(window, *value);
    }

    fn start_end(&self, info: InfoType<Self::Value>, window: &Window, space: &Space<Window>) -> (Self::Value, Self::Value) {
        let start = space.element_location(window).unwrap();
        let end = match info {
            InfoType::Delta(delta) => {
                delta + start
            },
            InfoType::Final(fin) => fin,
        };

        (start, end)
    }

    fn init() -> Self {
        Self
    }
}

#[derive(Debug)]
pub struct ResizeAnimation;

impl AnimationDriver for ResizeAnimation {
    type Value = Size<i32, Logical>;

    fn drive(&mut self, value: &Self::Value, window: &Window, _: &mut Space<Window>) {
        LayoutController::resize(window, ResizeType::Both(*value));
    }

    fn start_end(&self, info: InfoType<Self::Value>, window: &Window, _: &Space<Window>) -> (Self::Value, Self::Value) {
        let start = window.geometry().size;
        let end = match info {
            InfoType::Delta(mut delta) => {
                delta.w += start.w;
                delta.h += start.h;
                delta
            },
            InfoType::Final(fin) => fin,
        };

        (start, end)
    }

    fn init() -> Self {
        Self
    }
}

pub trait MutateDirections {
    fn horizontal(&mut self) -> &mut i32;
    fn vertical(&mut self) -> &mut i32;
}

impl<T> MutateDirections for Point<i32, T> {
    fn horizontal(&mut self) -> &mut i32 {
        &mut self.x
    }

    fn vertical(&mut self) -> &mut i32 {
        &mut self.y
    }
}

impl<T> MutateDirections for Size<i32, T> {
    fn horizontal(&mut self) -> &mut i32 {
        &mut self.w
    }

    fn vertical(&mut self) -> &mut i32 {
        &mut self.h
    }
}

pub trait Lerp: MutateDirections {
    fn lerp(start: &Self, end: &Self, point: f64) -> Self;
}

impl<T> Lerp for Point<i32, T> {
    fn lerp(start: &Self, end: &Self, point: f64) -> Self {
        let m = (*end - *start).to_f64();
        let q = start.to_f64();

        let y1 = m.x * point + q.x;
        let y2 = m.y * point + q.y;

        Point::new(y1.round_ties_even() as i32, y2.round_ties_even() as i32)
    }
}

impl<T> Lerp for Size<i32, T> {
    fn lerp(start: &Self, end: &Self, point: f64) -> Self {
        let mut size = Size::new(0, 0);

        let m1 = (end.w - start.w) as f64;
        let m2 = (end.h - start.h) as f64;

        let q = start.to_f64();

        let y1 = m1 * point + q.w;
        let y2 = m2 * point + q.h;

        size.w += y1.round_ties_even() as i32;
        size.h += y2.round_ties_even() as i32;

        size
    }
}