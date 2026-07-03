use std::{time::{Duration, Instant}};

use smithay::{reexports::{calloop::EventLoop, wayland_server::Display, x11rb::protocol::xkb::SelectEventsAuxNewKeyboardNotify}, utils::{Point, Rectangle, Size}};
use wayfleet::{layout::{controller::LayoutSettings, map::{Coordinate, Direction}}, state::State};

fn main() {
    let mut event_loop = EventLoop::<'static, State>::try_new().unwrap();
    let display = Display::<State>::new().unwrap();

    let mut state = State::new(&mut event_loop, display, LayoutSettings { rows: 3, columns: 5, cell_height: 200, cell_width: 200, area: Rectangle::new(Point::new(0, 0), Size::new(1473, 200))}, Size::new(1473, 976));

    wayfleet::winit::init_winit(&mut event_loop, &mut state).unwrap();

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket) };

    let mut last_done = Instant::now();
    
    let mut num = 0;
    let mut spawned = false;
    let mut moved = false;
    event_loop.run(None, &mut state, |state| {    
        if !spawned {
            std::process::Command::new("kitty").spawn().ok();
            std::process::Command::new("kitty").spawn().ok();
            std::process::Command::new("alacritty").spawn().ok();
            // std::process::Command::new("kitty").spawn().ok();
            spawned = true;
        }

        if last_done.elapsed() >= Duration::from_millis(2000) {
            last_done = Instant::now();
            state.layout.map.is_there_space_and_move(dbg!(&Coordinate { row: 1, column: num.min(4) }), &mut state.layout.space, Direction::Up);
            num +=1             
        }
    
        if last_done.elapsed() >= Duration::from_millis(1000) {
            if !moved {
                last_done = Instant::now();
                unsafe { state.layout.map.move_tile(&(0,2).into(), &(1,0).into(), &mut state.layout.space) };
                // unsafe { state.layout.map.move_tile(&(0,3).into(), &(1,1).into(), &mut state.layout.space) };
                state.layout.map.change_cells(&(1,0).into(), Direction::Right, &mut state.layout.space);
                moved = true;
            } else {
                state.layout.map.is_there_space_and_move(dbg!(&Coordinate { row: 0, column: num.min(4) }), &mut state.layout.space, Direction::Down);
            }
        }

        
    }).unwrap();

    // let mut last_done = Instant::now();
    // let mut thing = true;
    // let mut num = 0;
    // event_loop.run(None, &mut state, |_state| {
    //     if last_done.elapsed() >= Duration::from_millis(500) && num <= 18 {
    //         last_done = Instant::now();
    //         if thing {
    //             std::process::Command::new("alacritty").spawn().ok();
    //         } else {
    //             std::process::Command::new("kitty").spawn().ok();
    //         }
    //         thing = !thing;
    //         num+=1;
    //     }
        
    // }).unwrap();
}
