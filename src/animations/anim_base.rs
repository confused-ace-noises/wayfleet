use std::time::{Duration, Instant};

use smithay::desktop::{Space, Window};

use super::{drivers::AnimationDriver, easings::Easing, InfoType, drivers::Lerp};

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

    pub fn new(result: InfoType<T::Value>, window: Window, space: &Space<Window>, time: Duration, easing: Easing) -> Self {
        Self::new_with_wait(result, window, space, time, easing, 0)
    }

    pub fn new_with_wait(result: InfoType<T::Value>, window: Window, space: &Space<Window>, time: Duration, easing: Easing, wait: usize) -> Self {
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