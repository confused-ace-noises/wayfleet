use std::process::{Command, Stdio};

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Device, Event, InputBackend,
        InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::{FilterResult, KeysymHandle, ModifiersState, XkbConfig},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
    wayland::seat::WaylandFocus,
};
use wayfleet_config::keybinds::{Action, KeyBind, KeyCombo, Modifiers, Trigger};

use crate::{
    layout::{controller::{ForceSpawn, LayoutController}, map::coordinate::Direction}, state::State,
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
                    ) && state == KeyState::Pressed
                    {
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

                    if ButtonState::Pressed == button_state
                        && !pointer.is_grabbed()
                        && let Some(window) = self
                            .layout
                            .find_window(pointer.current_location().to_i32_round())
                            .cloned()
                    // .space
                    // .element_under(pointer.current_location())
                    // .map(|(w, l)| (w.clone(), l))
                    {
                        if self
                            .popups
                            .find_popup(&window.wl_surface().unwrap())
                            .is_some()
                        {
                            // only raise popups because otherwise map windows getting cropped to not
                            // bleed into the privileged area would raise above the privileged area
                            // and steal foucs. By not raising it, LayoutController always knows
                            // what z index every map and every privileged window is.
                            self.layout.space.raise_element(&window, true);
                        }

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

    let keycombo = KeyCombo { modifiers, trigger };

    let Some(keybind) = state
        .config
        .keybinds
        .keybinds
        .iter()
        .find(|x| x.combo.is_it(&keycombo, state.config.keybinds.mod_key))
    else {
        return FilterResult::Forward;
    };

    FilterResult::Intercept(keybind.clone())
}

pub fn handle_keybind(state: &mut State, keybind: KeyBind) {
    // let keybind = dbg!(keybind);

    let socket = state.socket.clone();
    let xwayland_display_number = state.xwayland_display_number.clone();
    let spawn = |command: &mut Command| {
        let cmd = command
            .env("WAYLAND_DISPLAY", &socket)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        
        if let Some(disp_n) = &xwayland_display_number {
            cmd
                .env("DISPLAY", disp_n);
        }

        let _ = cmd.spawn();
    };

    for action in keybind.actions {
        match action {
            // * move focusthread_local! {}
            Action::MoveFocusUp => LayoutController::move_focus(state, Direction::Up),
            Action::MoveFocusDown => LayoutController::move_focus(state, Direction::Down),
            Action::MoveFocusRight => LayoutController::move_focus(state, Direction::Right),
            Action::MoveFocusLeft => LayoutController::move_focus(state, Direction::Left),

            // * move (map) or swap (map & privileged) window
            Action::MoveOrSwapUp => state.layout.swap_focused(Direction::Up),
            Action::MoveOrSwapDown => state.layout.swap_focused(Direction::Down),
            Action::MoveOrSwapRight => state.layout.swap_focused(Direction::Right),
            Action::MoveOrSwapLeft => state.layout.swap_focused(Direction::Left),

            Action::MoveFocusedToOtherArea => state.layout.move_focused_to_other_area(),

            // * move and shift in map, absorb/eject from columns in privileged
            Action::PushLateralRight => state.layout.push_privileged_laterally(Direction::Right),
            Action::PushLateralLeft => state.layout.push_privileged_laterally(Direction::Left),
            
            // * resize window
            Action::SetPrivilegedWindowHeight(a) => state.layout.resize_focused_window_privileged(a, false),
            Action::SetPrivilegedWindowWidth(a)  => state.layout.resize_focused_window_privileged(a, true),
            
            Action::MapResizeAddUp       => { state.layout.change_cell_size_map_if_focused(Direction::Up,    false); },
            Action::MapResizeAddDown     => { state.layout.change_cell_size_map_if_focused(Direction::Down,  false); },
            Action::MapResizeAddRight    => { state.layout.change_cell_size_map_if_focused(Direction::Right, false); },
            Action::MapResizeAddLeft     => { state.layout.change_cell_size_map_if_focused(Direction::Left,  false); },
            Action::MapResizeRemoveUp    => { state.layout.change_cell_size_map_if_focused(Direction::Up,    true);  },
            Action::MapResizeRemoveDown  => { state.layout.change_cell_size_map_if_focused(Direction::Down,  true);  },
            Action::MapResizeRemoveRight => { state.layout.change_cell_size_map_if_focused(Direction::Right, true);  },
            Action::MapResizeRemoveLeft  => { state.layout.change_cell_size_map_if_focused(Direction::Left,  true);  },
            
            // * resize cells
            Action::SetMapCellHeight(amount) => state.layout.resize_cells_map(amount, true),
            Action::SetMapCellWidth(amount) => state.layout.resize_cells_map(amount, false),

            Action::SetPrivilegedCellHeight(amount) => {
                state.layout.resize_column_height_privileged(amount)
            }

            Action::SetFocusedCellHeight(amount) => state.layout.resize_cells_focused(amount, true),
            Action::SetFocusedCellWidth(amount) => state.layout.resize_cells_focused(amount, false),

            // FIXME: this is hortrbile.
            Action::MapAddColumn => {
                state.layout.map.columns += 1;
                state.layout.map.recalculate_available();
            },
            Action::MapAddRow => {
                state.layout.map.rows += 1;
                state.layout.map.recalculate_available();
            }
            Action::MapRemoveColumn => {
                let mut x = 0;
                let mut iter = std::iter::from_fn(|| {
                    let out = state.layout.map.map.get(x)?.last()?;
                    x += 1;
                    Some(out)
                });

                if !iter.any(|x| x.is_some()) {
                    state.layout.map.columns -= 1;
                    state.layout.map.recalculate_available();
                }

            }
            Action::MapRemoveRow => {
                if let Some(last_row) = state.layout.map.map.last() && !last_row.iter().any(|el| el.is_some()) {
                    state.layout.map.rows -= 1;
                    state.layout.map.recalculate_available();
                }
            }

            // * spawning
            // TODO: reap child processes or find a way of using double forking
            // ? note: current fix has just been by setting the action for
            // ?       SICHLD to be SA_NOCLDWAIT, which just closes the
            // ?       child process w/o making zombies
            Action::Spawn(items) => {
                let (command, args) = items.split_first().unwrap();

                spawn(Command::new(command).args(args));
            }
            Action::SpawnSh(string) => {
                spawn(Command::new("sh").arg("-c").arg(string));
            }

            Action::SpawnPrivileged(items) => {
                state.layout.forced_windows.push_back(ForceSpawn::Priv);

                let (command, args) = items.split_first().unwrap();

                spawn(
                    Command::new(command)
                        .args(args)
                );
            }
            Action::SpawnPrivilegedSh(string) => {
                state.layout.forced_windows.push_back(ForceSpawn::Priv);

                spawn(Command::new("sh").arg("-c").arg(string))
            }

            Action::SpawnMap(items) => {
                state.layout.forced_windows.push_back(ForceSpawn::Map);

                let (command, args) = items.split_first().unwrap();

                spawn(
                    Command::new(command)
                        .args(args)
                );
            }
            Action::SpawnMapSh(string) => {
                state.layout.forced_windows.push_back(ForceSpawn::Map);

                spawn(Command::new("sh").arg("-c").arg(string))
            }

            Action::CloseWindow => {
                // state.layout.
                let current_focus = state.layout.currently_focused().cloned();

                if let Some(ref focued) = current_focus {
                    LayoutController::remove(state, focued);

                    match focued.underlying_surface() {
                        smithay::desktop::WindowSurface::Wayland(toplevel) => toplevel.send_close(),
                        smithay::desktop::WindowSurface::X11(x11) => x11.close().unwrap_or_default(),
                    }
                }
            }
            Action::Quit => {
                state.loop_signal.stop();
            }
            Action::None => {}

            // * Diag
            Action::DumpMap => println!("dump-map was called: {:?}", state.layout.map),
            Action::DumpPrivileged => {
                println!("dump-privileged was called: {:?}", state.layout.privileged)
            }
            Action::DumpLayout => println!("dump-layout was called: {:?}", state.layout),
            
        }
    }
}
