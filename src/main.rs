use std::{time::{Duration, Instant}};

use smithay::{reexports::{calloop::EventLoop, wayland_server::Display}, utils::{Point, Rectangle, Size}};
use wayfleet::{layout::{controller::LayoutSettings, map::{Coordinate, Direction}}, state::State};
fn main() {
    let mut event_loop = EventLoop::<'static, State>::try_new().unwrap();
    let display = Display::<State>::new().unwrap();

    let mut state = State::new(&mut event_loop, display, LayoutSettings { rows: 1, columns: 1, cell_height: 200, cell_width: 200, area: Rectangle::new(Point::new(0, 0), Size::new(1472, 200))}, Size::new(1473, 976));

    wayfleet::winit::init_winit(&mut event_loop, &mut state).unwrap();

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket) };

    let mut last_done = Instant::now();
    
    let mut num = 0;
    let mut spawned = false;
    event_loop.run(None, &mut state, |state| {    
        println!("--------");
        if !spawned {
            std::process::Command::new("kitty").spawn().ok();
            std::process::Command::new("kitty").spawn().ok();
            std::process::Command::new("alacritty").spawn().ok();
            // std::process::Command::new("kitty").spawn().ok();
            spawned = true;
        }

        if last_done.elapsed() >= Duration::from_millis(1000) {
            last_done = Instant::now();
            state.layout.privileged.swap((0,0), &mut state.layout.space, Direction::Right);
        } 

        // if last_done.elapsed() >= Duration::from_millis(2500) {
        //     last_done = Instant::now();
        //     state.layout.map.is_there_space_and_move(&Coordinate { row: 1, column: 0 }, &mut state.layout.space, Direction::Right);
        //     num +=1             
        // }
    
        // if last_done.elapsed() >= Duration::from_millis(1500) {
        //     if moved.is_none() {
        //         last_done = Instant::now();
        //         // unsafe { state.layout.map.move_tile(&(0,2).into(), &(1,0).into(), &mut state.layout.space) };
        //         // unsafe { state.layout.map.move_tile(&(0,3).into(), &(1,1).into(), &mut state.layout.space) };
        //         println!("done");
        //         state.layout.map.change_cells(&(0,0).into(), Direction::Down, false, &mut state.layout.space);
        //         moved = Some(false);
        //     } else if !moved.unwrap() {
        //         dbg!(&state.layout.map.first_available);
        //         std::process::Command::new("alacritty").spawn().ok();
        //         state.layout.map.change_cells(&(0,0).into(), Direction::Down, true, &mut state.layout.space);
        //         moved = Some(true)
        //     }
        //     // if last_done.elapsed() >= Duration::from_millis(1500) {
        //     // }
        // }

        state.layout.tick_animation();
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
