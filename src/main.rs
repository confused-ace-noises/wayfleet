use std::time::Instant;
use miette::Result;
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use wayfleet::{startup_spawns, state::State};
use wayfleet_config::Config;

const CONFIG_FILE: &str = "config.kdl";

fn main() -> Result<()> {
    let config = Config::parse(CONFIG_FILE)?;

    let mut event_loop = EventLoop::<'static, State>::try_new().unwrap();
    let display = Display::<State>::new().unwrap();

    let mut state = wayfleet::winit::init_winit(&mut event_loop, display, config).unwrap();

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket) };

    let _last_done = Instant::now();
    
    let action = SigAction::new(SigHandler::SigDfl, SaFlags::SA_NOCLDWAIT, SigSet::empty());

    unsafe { sigaction(Signal::SIGCHLD, &action).unwrap() };

    startup_spawns(&state.socket);
    event_loop.run(None, &mut state, |state| {    
        state.layout.tick_animation();
    }).unwrap();

    Ok(())
}