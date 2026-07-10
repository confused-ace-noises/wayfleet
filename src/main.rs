use std::{time::{Duration, Instant}};
use miette::Result;
use smithay::{reexports::{calloop::EventLoop, wayland_server::Display}, utils::{Point, Rectangle, Size}};
use wayfleet::{layout::{controller::LayoutSettings, map::{Coordinate, Direction}}, state::State};
use wayfleet_config::{Config, error::ConfigError};

const CONFIG_FILE: &str = "config.toml";

fn main() -> Result<()> {
    let config = Config::parse(CONFIG_FILE)?;

    let mut event_loop = EventLoop::<'static, State>::try_new().unwrap();
    let display = Display::<State>::new().unwrap();

    let mut state = wayfleet::winit::init_winit(&mut event_loop, display, config).unwrap();

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket) };

    let mut last_done = Instant::now();
    
    let mut num = 0;
    let mut spawned = None;
    let mut did_mod = false;
    event_loop.run(None, &mut state, |state| {    
        println!("--------");
        if spawned.is_none() {
            std::process::Command::new("alacritty").spawn().ok();
            std::process::Command::new("kitty").spawn().ok();
            std::process::Command::new("alacritty").spawn().ok();
            std::process::Command::new("kitty").spawn().ok();
            spawned = Some(false);
        }

        // * map swap demo
        if let Some(false) = spawned && last_done.elapsed() >= Duration::from_millis(1000) {
            state.layout.map.change_cells(&(0, 0).into(), Direction::Down, false, &mut state.layout.space);
            state.layout.map.swap_or_move(&(0, 1).into(), Direction::Down, &mut state.layout.space);
            state.layout.map.swap_or_move(&(0, 2).into(), Direction::Left, &mut state.layout.space);
            // state.layout.map.change_cells(&(1, 1).into(), Direction::Down, false, &mut state.layout.space);
            
            spawned = Some(true);
            println!("{:#?}", state.layout.map.map);
        }
        
        if !did_mod && last_done.elapsed() >= Duration::from_millis(1200) {
            state.layout.map.change_cells(&(1, 1).into(), Direction::Right, false, &mut state.layout.space);
            did_mod = true;
        }
        
        if last_done.elapsed() >= Duration::from_millis(1500) {
            last_done = Instant::now();
            state.layout.map.swap_or_move(&(1,0).into(), Direction::Right, &mut state.layout.space);
            println!("swapped: {:#?}", state.layout.map.map);
        }
        // * map swap demo

        // * privileged swap demo
        // if last_done.elapsed() >= Duration::from_millis(1000) {
        //     last_done = Instant::now();
        //     state.layout.privileged.swap((0,0), Direction::Right, &mut state.layout.space);
        // }
        // * privileged swap demo

        state.layout.tick_animation();
    }).unwrap();

    Ok(())
}
