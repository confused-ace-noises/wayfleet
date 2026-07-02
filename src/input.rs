use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, Axis, AxisSource, ButtonState, Device, Event, InputBackend,
            InputEvent, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
        },
    },
    input::{
        keyboard::FilterResult,
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::SERIAL_COUNTER,
    wayland::seat::WaylandFocus,
};

use crate::state::State;

impl State {
    pub fn run_input<I: InputBackend>(&mut self, input: InputEvent<I>) {
        match input {
            InputEvent::DeviceAdded { device } => {
                if device.has_capability(smithay::backend::input::DeviceCapability::Keyboard) {
                    self.seat.add_keyboard(Default::default(), 200, 25).unwrap();
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
                    kb.input(
                        self,
                        keycode,
                        state,
                        SERIAL_COUNTER.next_serial(),
                        event.time_msec(),
                        |_, _, _| FilterResult::<()>::Forward,
                    );
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
                        .find_window_pos(pos.to_i32_round())
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
                    let keyboard = self.seat.get_keyboard();

                    let serial = SERIAL_COUNTER.next_serial();

                    let button = event.button_code();

                    let button_state = event.state();

                    if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
                        if let Some(window) = self
                            .layout
                            .find_window(pointer.current_location().to_i32_round())
                            .cloned()
                        {
                            self.layout.space.raise_element(&window, true);
                            keyboard.inspect(|kb| {
                                kb.set_focus(
                                    self,
                                    Some(window.toplevel().unwrap().wl_surface().clone()),
                                    serial,
                                )
                            });
                            self.layout.space.elements().for_each(|window| {
                                window.toplevel().unwrap().send_pending_configure();
                            });
                        } else {
                            self.layout.space.elements().for_each(|window| {
                                window.set_activated(false);
                                window.toplevel().unwrap().send_pending_configure();
                            });
                            keyboard.inspect(|kb| kb.set_focus(self, Option::<WlSurface>::None, serial));
                        }
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
