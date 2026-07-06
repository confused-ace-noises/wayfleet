use std::{collections::HashMap, sync::{Arc, RwLock}, time::{Duration, Instant}};

use smithay::{desktop::{Space, Window}, utils::{Logical, Point, Size}};

use crate::layout::controller::{LayoutController, ResizeType};

#[derive(Debug, derive_more::Deref, derive_more::DerefMut, Clone)]
pub struct AnimationHandle(pub Arc<RwLock<AnimationController>>);

#[derive(Debug)]
pub struct AnimationController {
    pub pending: HashMap<Window, Vec<Animation>>,
    pub frequency: Duration,
    pub last: Instant,
}

impl AnimationController {
    pub fn new(freq: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            frequency: freq,
            last: Instant::now(),
        }
    }

    pub fn schedule(&mut self, anim: Animation) {
        let window = anim.window();
        self.pending.entry(window.clone()).or_insert(vec![]).push(anim);
    }

    pub fn tick(&mut self, space: &mut Space<Window>) {
        let now = Instant::now();

        if now.duration_since(self.last) >= self.frequency {
            self.last = now;
            self.pending.retain(|_, v| {
                v.retain_mut(|anim| !anim.tick(space));
    
                !v.is_empty()
            });
        }
    }
}

#[derive(Debug)]
pub enum Animation {
    Move(AnimationBase<MoveAnimation>),
    Resize(AnimationBase<ResizeAnimation>),
}

impl Animation {
    pub fn window(&self) -> &Window {
        match self {
            Animation::Move(animation_base) => &animation_base.window,
            Animation::Resize(animation_base) => &animation_base.window,
        }
    }

    pub fn tick(&mut self, space: &mut Space<Window>) -> bool {
        match self {
            Animation::Move(animation_base) => animation_base.tick(space),
            Animation::Resize(animation_base) => animation_base.tick(space),
        }
    }

    pub fn last_val_move(&self) -> Option<<MoveAnimation as AnimationDriver>::Value> {
        match self {
            Animation::Move(animation_base) => animation_base.last_value,
            Animation::Resize(_) => None,
        }
    }

    pub fn last_val_resize(&self) -> Option<<ResizeAnimation as AnimationDriver>::Value> {
        match self {
            Animation::Resize(animation_base) => animation_base.last_value,
            Animation::Move(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum InfoType<T> {
    Delta(T),
    Final(T),
    StartEnd(T, T),
}

#[derive(Debug)]
pub struct AnimationBase<T: AnimationDriver> {
    pub window: Window,
    pub total_time: Duration,
    pub start_time: Instant,
    pub easing: Easing,
    pub start: T::Value,
    pub end: T::Value,
    pub driver: T,
    pub last_value: Option<T::Value>,
    pub wait: usize
}

impl<T: AnimationDriver> AnimationBase<T> {
    pub fn new_with_time(result: InfoType<T::Value>, window: Window, space: &Space<Window>, time: Duration, easing: Easing, start_time: Instant, wait: usize) -> Self {
        let driver = T::init();
        let (start, end) = driver.start_end(result, &window, space);

        Self {
            window,
            total_time: time,
            start_time,
            easing,
            start,
            end,
            driver,
            last_value: None,
            wait,
        }
    }

    pub fn new(result: InfoType<T::Value>, window: Window, space: &Space<Window>, time: Duration, easing: Easing, wait: usize) -> Self {
        let now = Instant::now();
        Self::new_with_time(result, window, space, time, easing, now, wait)
    }

    pub fn tick(&mut self, space: &mut Space<Window>) -> bool {
        if self.wait > 0 {
            self.wait -= 1;
            self.start_time = Instant::now();
            return false;
        }

        let now = Instant::now();
        let partial = now.duration_since(self.start_time);

        let time_norm = (partial.as_millis() as f64 / self.total_time.as_millis() as f64).min(1.);

        let mut finish = false;

        if time_norm >= 0.99 {
            finish = true;
        }

        if !finish {
            let state_norm = (self.easing.func())(time_norm);
    
            let lerped = T::Value::lerp(&self.start, &self.end, state_norm);
            
            
            self.driver.drive(&lerped, &self.window, space);
            self.last_value = Some(lerped);
        } else {
            self.driver.drive(&self.end, &self.window, space);
            self.last_value = Some(self.end.clone())
        }
        
        finish
    }
}

pub trait AnimationDriver {
    type Value: Lerp + Clone;
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
        let mut start = space.element_location(window).unwrap();
        let end = match info {
            InfoType::Delta(delta) => {
                delta + start
            },
            InfoType::Final(fin) => fin,
            InfoType::StartEnd(spec_start, end) => {
                start = spec_start;
                end
            }
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
        let mut start = window.geometry().size;
        let end = match info {
            InfoType::Delta(mut delta) => {
                delta.w += start.w;
                delta.h += start.h;
                delta
            },
            InfoType::Final(fin) => fin,
            InfoType::StartEnd(spec_start, end) => {
                start = spec_start;
                end
            }
        };

        (start, end)
    }

    fn init() -> Self {
        Self
    }
}

#[derive(Debug)]
pub enum Easing {
    Linear,
    EaseInOut,
    EaseOutBack,
}

impl Easing {
    pub fn func(&self) -> fn(f64) -> f64 {
        match self {
            Easing::Linear => Self::linear,
            Easing::EaseInOut => Self::ease_in_out,
            Easing::EaseOutBack => Self::ease_out_back,
        }
    }

    fn linear(x: f64) -> f64 {
        x
    }

    fn ease_in_out(x: f64) -> f64 {
        if x < 0.5 {
            4. * x * x * x
        } else {
            1. - (-2. * x + 2.).powi(3) / 2.
        }
    }

    fn ease_out_back(x: f64) -> f64 {
        let c1 = 1.70158;
        let c3 = c1 + 1.;

        1. + c3 * (x - 1.).powi(3) + c1 * (x - 1.).powi(2)
    }
}

pub trait Lerp {
    fn lerp(start: &Self, end: &Self, point: f64) -> Self;
}

impl<T> Lerp for Point<i32, T> {
    fn lerp(start: &Self, end: &Self, point: f64) -> Self {
        let m = (*end - *start).to_f64();
        let q = start.to_f64();

        let y1 = m.x * point + q.x;
        let y2 = m.y * point + q.y;

        dbg!(Point::new(y1.round() as i32, y2.round() as i32))
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

        size.w += y1.round() as i32;
        size.h += y2.round() as i32;

        dbg!(size)
    }
}