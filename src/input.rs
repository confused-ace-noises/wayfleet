use smithay::backend::{input::InputEvent, winit::{WinitEvent, WinitInput}};

use crate::state::State;

impl State {
    pub fn run_input(&mut self, input: InputEvent<WinitInput>) {}
}