use core::fmt;
use std::{any::{Any, TypeId}, collections::HashMap, fmt::Debug, sync::{Arc, RwLock}, time::{Duration, Instant}};

use smithay::desktop::{Space, Window};

pub mod drivers;
pub mod easings;
pub mod anim_base;

pub use drivers::*;
pub use easings::*;
pub use anim_base::*;

#[derive(Debug)]
pub enum InfoType<T> {
    Delta(T),
    Final(T),
}

pub enum DynInfoType {
    Delta(i32, i32),
    Final(i32, i32),
    // StartEnd(i32, T),
}

#[derive(Debug, derive_more::Deref, derive_more::DerefMut, Clone)]
pub struct AnimationHandle(pub Arc<RwLock<AnimationController>>);

#[derive(Debug)]
pub struct AnimationController {
    #[allow(private_interfaces)]
    pub running: HashMap<Window, Vec<Box<dyn AnimationErased>>>,
    pub frequency: Duration,
    pub last: Instant,
}

#[allow(unused)]
pub(super) trait DynWaitingAnim: Debug {
    fn into_anim(self: Box<Self>, window: Window, space: &Space<Window>) -> Box<dyn AnimationErased>;

    fn driver_type(&self) -> TypeId;
    
    fn duration(&self) -> &Duration;
    fn easing(&self) -> Easing;
    fn result(&self) -> DynInfoType;
}

#[derive(Debug)]
pub(super) struct WaitingAnimation<A: AnimationDriver + fmt::Debug + 'static> {
    result: InfoType<A::Value>,
    duration: Duration,
    easing: Easing
}

impl<A> DynWaitingAnim for WaitingAnimation<A> 
where 
    A: AnimationDriver + fmt::Debug + 'static
{
    fn into_anim(self: Box<Self>, window: Window, space: &Space<Window>) -> Box<dyn AnimationErased> {
        let WaitingAnimation { result, duration, easing } = *self;

        let base_anim = AnimationBase::<A>::new(result, window, space, duration, easing);

        Box::new(base_anim)
    }

    fn driver_type(&self) -> TypeId {
        TypeId::of::<A>()
    }
    
    fn duration(&self) -> &Duration {
        &self.duration
    }
    
    fn easing(&self) -> Easing {
        self.easing
    }
    
    fn result(&self) -> DynInfoType {
        match &self.result {
            InfoType::Delta(x) => {
                let x = x.clone().into();
                DynInfoType::Delta(x.0, x.1)
            },
            InfoType::Final(x) => {
                let x = x.clone().into();
                DynInfoType::Final(x.0, x.1)
            },
        }
    }
}

impl AnimationController {
    pub fn new(freq: Duration) -> Self {
        Self {
            running: HashMap::new(),
            frequency: freq,
            last: Instant::now(),
        }
    }

    pub fn schedule<A>(&mut self, result: InfoType<A::Value>, window: Window, space: &Space<Window>, time: Duration, easing: Easing) 
    where 
        A: AnimationDriver + fmt::Debug + 'static 
    {
        let id = TypeId::of::<A>();

        let vec = self.running.entry(window.clone()).or_insert(vec![]);
        if let Some(found) = vec.iter_mut().find(|x| x.driver_type() == id) {
            found.extend_anim(Box::new(WaitingAnimation::<A> { result, duration: time, easing }));
        } else {
            let x = AnimationBase::<A>::new(result, window, space, time, easing);
            vec.push(Box::new(x));
        }
    }

    pub fn schedule_specific<A>(&mut self, base: AnimationBase<A>)
    where 
        A: AnimationDriver + fmt::Debug + 'static,
    {
        let window = &base.window;
        self.running.entry(window.clone()).or_insert(vec![]).push(Box::new(base));
    }

    pub fn tick(&mut self, space: &mut Space<Window>) {
        let now = Instant::now();

        if now.duration_since(self.last) >= self.frequency {
            self.last = now;
            self.running.retain(|_, v| {                
                v.retain_mut(|anim| !anim.tick(space));

                !v.is_empty()
            });
        }
    }
}

#[allow(unused)]
pub(super) trait AnimationErased: fmt::Debug + Send + Sync {
    fn window(&self) -> &Window;
    fn tick(&mut self, space: &mut Space<Window>) -> bool;

    fn as_any(&self) -> &dyn Any;
    fn driver_type(&self) -> TypeId;
    fn extend_anim(&mut self, waiting: Box<dyn DynWaitingAnim>);
}

impl<T> AnimationErased for AnimationBase<T>
where
    T: fmt::Debug + AnimationDriver + 'static + Send + Sync,
    T::Value: fmt::Debug,
{
    fn window(&self) -> &Window {
        &self.window
    }

    fn tick(&mut self, space: &mut Space<Window>) -> bool {
        self.tick(space)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn driver_type(&self) -> TypeId {
        TypeId::of::<T>()
    }
    
    fn extend_anim(&mut self, waiting: Box<dyn DynWaitingAnim>) {
        let result = waiting.result();

        if self.last_value.is_some() {
            self.start = self.last_value.take().unwrap();
        }

        self.start_time = Instant::now();
        self.total_time = *waiting.duration();
        self.easing = waiting.easing();

        // TODO: maybe adding a thing to check if the previous 
        // animation was Delta so that you can add Final to that
        // instead of overwriting it?
        match result {
            DynInfoType::Delta(h, v) => {
                *self.end.horizontal() += h;
                *self.end.vertical() += v;
            },
            DynInfoType::Final(h, v) => {
                *self.end.horizontal() = h;
                *self.end.vertical() = v;
            },
        }
    }
}
