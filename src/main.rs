use std::{thread::sleep, time::{Duration, Instant}};

use smithay::{backend, reexports::{calloop::EventLoop, wayland_server::Display}, utils::{Point, Rectangle, Size}};
use wayfleet::{layout::LayoutSettings, state::State};

fn main() {
    let mut event_loop = EventLoop::<'static, State>::try_new().unwrap();
    let display = Display::<State>::new().unwrap();

    let mut state = State::new(&mut event_loop, display, LayoutSettings { rows: 4, columns: 4, cell_height: 200, cell_width: 200, area: Rectangle::new(Point::new(0, 0), Size::new(1000, 200))});

    wayfleet::winit::init_winit(&mut event_loop, &mut state).unwrap();

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket) };

    let mut last_done = Instant::now();
    event_loop.run(None, &mut state, |_state| {
        if last_done.elapsed() >= Duration::from_secs(1) {
            last_done = Instant::now();
            std::process::Command::new("weston-terminal").spawn().ok();
        }
        
    }).unwrap();
}
