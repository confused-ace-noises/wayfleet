use std::process::{Command, Stdio};

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Device, Event, InputBackend, InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    }, input::{
        keyboard::{FilterResult, KeysymHandle, ModifiersState, XkbConfig}, pointer::{AxisFrame, ButtonEvent, MotionEvent},
    }, utils::SERIAL_COUNTER, wayland::seat::WaylandFocus,
};
use wayfleet_config::keybinds::{KeyBind, KeyCombo, Modifiers, Trigger};

use crate::{
    layout::{controller::LayoutController, map::coordinate::Direction},
    state::State,
};

impl State {
    pub fn run_input<I: InputBackend>(&mut self, input: InputEvent<I>) {
        match input {
            InputEvent::DeviceAdded { device } => {
                if device.has_capability(smithay::backend::input::DeviceCapability::Keyboard) {
                    let xkb_config = XkbConfig {
                        layout: &self.config.input.keyboard.layout,
                        ..Default::default()
                    };

                    self.seat.add_keyboard(xkb_config, 200, 25).unwrap();
                }

                if device.has_capability(smithay::backend::input::DeviceCapability::Pointer) {
                    self.seat.add_pointer();
                }
            }
            InputEvent::DeviceRemoved { device } => {
                if device.has_capability(smithay::backend::input::DeviceCapability::Keyboard) {
                    self.seat.remove_keyboard();
                }

                if device.has_capability(smithay::backend::input::DeviceCapability::Pointer) {
                    self.seat.remove_pointer();
                }
            }
            InputEvent::Keyboard { event } => {
                if let Some(kb) = self.seat.get_keyboard() {
                    let keycode = event.key_code();
                    let state = event.state();
                    if let Some(bind) = kb.input(
                        self,
                        keycode,
                        state,
                        SERIAL_COUNTER.next_serial(),
                        event.time_msec(),
                        kb_filter,
                    )
                    && state == KeyState::Pressed {
                        handle_keybind(self, bind);
                    }
                }
            }
            InputEvent::PointerMotion { .. } => {}
            InputEvent::PointerMotionAbsolute { event } => {
                if let Some(pointer) = self.seat.get_pointer() {
                    let output = self.layout.space.outputs().next().unwrap();

                    let output_geo = self.layout.space.output_geometry(output).unwrap();

                    let pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                    let serial = SERIAL_COUNTER.next_serial();

                    let under = self
                        .layout
                        .find_window_pos(pos)
                        .and_then(|(w, p)| Some((w.wl_surface()?.into_owned(), p.to_f64())));

                    pointer.motion(
                        self,
                        under,
                        &MotionEvent {
                            location: pos,
                            serial,
                            time: event.time_msec(),
                        },
                    );
                    pointer.frame(self);
                }
            }
            InputEvent::PointerButton { event } => {
                if let Some(pointer) = self.seat.get_pointer() {
                    // let keyboard = self.seat.get_keyboard();

                    let serial = SERIAL_COUNTER.next_serial();

                    let button = event.button_code();

                    let button_state = event.state();

                    if ButtonState::Pressed == button_state && !pointer.is_grabbed()
                        && let Some(window) = self
                            .layout
                            .find_window(pointer.current_location().to_i32_round())
                            .cloned()
                        // .space
                        // .element_under(pointer.current_location())
                        // .map(|(w, l)| (w.clone(), l))
                    {
                        
                        self.layout.space.raise_element(&window, false); // gets activated by layout
                        // TODO: hanle popups
                        LayoutController::new_focus(self, window);
                    };

                    pointer.button(
                        self,
                        &ButtonEvent {
                            button,
                            state: button_state,
                            serial,
                            time: event.time_msec(),
                        },
                    );
                    pointer.frame(self);
                }
            }
            InputEvent::PointerAxis { event } => {
                let source = event.source();

                let horizontal_amount = event.amount(Axis::Horizontal).unwrap_or_else(|| {
                    event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.
                });
                let vertical_amount = event.amount(Axis::Vertical).unwrap_or_else(|| {
                    event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.
                });
                let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
                let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

                let mut frame = AxisFrame::new(event.time_msec()).source(source);
                if horizontal_amount != 0.0 {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);
                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.v120(Axis::Horizontal, discrete as i32);
                    }
                }
                if vertical_amount != 0.0 {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.v120(Axis::Vertical, discrete as i32);
                    }
                }

                if source == AxisSource::Finger {
                    if event.amount(Axis::Horizontal) == Some(0.0) {
                        frame = frame.stop(Axis::Horizontal);
                    }
                    if event.amount(Axis::Vertical) == Some(0.0) {
                        frame = frame.stop(Axis::Vertical);
                    }
                }

                let pointer = self.seat.get_pointer().unwrap();
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}

pub fn kb_filter(
    state: &mut State,
    modifiers: &ModifiersState,
    keysym: KeysymHandle<'_>,
) -> FilterResult<KeyBind> {
    let mut modifiers: Modifiers = modifiers.into();

    if (state.config.keybinds.mod_key & modifiers).bits() > 0 {
        modifiers |= Modifiers::DEFAULT;
    }


    let Some(raw) = keysym.raw_latin_sym_or_raw_current_sym() else {
        return FilterResult::Forward;
    };

    let trigger = Trigger::Keysym(raw);

    println!("{:?}", trigger);

    let keycombo = KeyCombo { modifiers, trigger };

    let Some(keybind) = state
        .config
        .keybinds
        .keybinds
        .iter()
        .find(|x| x.combo.is_it(&keycombo, state.config.keybinds.mod_key) )
    else {
        return FilterResult::Forward;
    };

    println!("match!!!!");

    FilterResult::Intercept(keybind.clone())
}

pub fn handle_keybind(state: &mut State, keybind: KeyBind) {
    let keybind = dbg!(keybind);

    match keybind.action {
        // * move focus
        wayfleet_config::keybinds::Action::MoveFocusUp => {
            LayoutController::move_focus(state, Direction::Up)
        }
        wayfleet_config::keybinds::Action::MoveFocusDown => {
            LayoutController::move_focus(state, Direction::Down)
        }
        wayfleet_config::keybinds::Action::MoveFocusRight => {
            LayoutController::move_focus(state, Direction::Right)
        }
        wayfleet_config::keybinds::Action::MoveFocusLeft => {
            println!("moving focus");
            LayoutController::move_focus(state, Direction::Left)
        }

        // * move (map) or swap (map & privileged) window
        wayfleet_config::keybinds::Action::MoveOrSwapUp => state.layout.swap_focused(Direction::Up),
        wayfleet_config::keybinds::Action::MoveOrSwapDown => state.layout.swap_focused(Direction::Down),
        wayfleet_config::keybinds::Action::MoveOrSwapRight => state.layout.swap_focused(Direction::Right),
        wayfleet_config::keybinds::Action::MoveOrSwapLeft => state.layout.swap_focused(Direction::Left),

        // * move and shift in map, absorb/eject from columns in privileged
        wayfleet_config::keybinds::Action::PushLateralRight => {
            println!("doing right");    
            state.layout.push_privileged_laterally(Direction::Right)
        
        },
        wayfleet_config::keybinds::Action::PushLateralLeft  => {
            println!("doing left");    
            state.layout.push_privileged_laterally(Direction::Left)
        },

        // TODO: reap child processes or find a way of using double forking
        // ? note: current fix has just been by setting the action for 
        // ?       SICHLD to be SA_NOCLDWAIT, which just closes the 
        // ?       child process w/o making zombies
        wayfleet_config::keybinds::Action::Spawn(items) => {
            let (command, args) = items.split_first().unwrap();

            let _ = Command::new(command)
                .args(args)
                .env("WAYLAND_DISPLAY", &state.socket)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        wayfleet_config::keybinds::Action::SpawnSh(string) => {
            let _ = Command::new("sh")
                .arg("-c")
                .arg(string)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }

        wayfleet_config::keybinds::Action::CloseWindow => {
            // state.layout.
            let current_focus = state.layout.currently_focused().cloned();

            if let Some(ref focued) = current_focus {
                LayoutController::remove(state, focued);
            }
        },
        wayfleet_config::keybinds::Action::Quit => {
            state.loop_signal.stop();
        },
        wayfleet_config::keybinds::Action::None => {},
        
        // * Diag
        wayfleet_config::keybinds::Action::DumpMap => println!("dump-map was called: {:?}", state.layout.map),
        wayfleet_config::keybinds::Action::DumpPrivileged => println!("dump-privileged was called: {:?}", state.layout.privileged),
        wayfleet_config::keybinds::Action::DumpLayout => println!("dump-layout was called: {:?}", state.layout),
    }
}
