use std::{time::{Duration, Instant}};

use smithay::{reexports::{calloop::EventLoop, wayland_server::Display}, utils::{Point, Rectangle, Size}};
use wayfleet::{layout::LayoutSettings, state::State};

fn main() {
    let mut event_loop = EventLoop::<'static, State>::try_new().unwrap();
    let display = Display::<State>::new().unwrap();

    let mut state = State::new(&mut event_loop, display, LayoutSettings { rows: 3, columns: 5, cell_height: 200, cell_width: 200, area: Rectangle::new(Point::new(0, 0), Size::new(1473, 200))}, Size::new(1473, 976));

    wayfleet::winit::init_winit(&mut event_loop, &mut state).unwrap();

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket) };

    let mut last_done = Instant::now();
    let mut thing = false;
    let mut num = 0;
    event_loop.run(None, &mut state, |_state| {
        if last_done.elapsed() >= Duration::from_millis(500) && num <= 18 {
            last_done = Instant::now();
            if thing {
                std::process::Command::new("kitty").spawn().ok();
            } else {
                std::process::Command::new("alacritty").spawn().ok();
            }
            thing = !thing;
            num+=1;
        }
        
    }).unwrap();
}
