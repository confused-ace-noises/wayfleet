use smithay::{backend, reexports::{calloop::EventLoop, wayland_server::Display}};
use wayfleet::state::State;

fn main() {
    let event_loop = EventLoop::<'static, State>::try_new().unwrap();
    let display = Display::<State>::new().unwrap();
}
