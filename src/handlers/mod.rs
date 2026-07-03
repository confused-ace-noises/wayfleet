use smithay::{input::{SeatHandler, dnd::{DnDGrab, DndGrabHandler, GrabType}, pointer::Focus}, reexports::wayland_server::{backend::ClientData, protocol::wl_surface::WlSurface}, wayland::{buffer::BufferHandler, compositor::CompositorClientState, output::OutputHandler, selection::{SelectionHandler, data_device::{DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler}}, shm::{ShmHandler, ShmState}}};

use crate::state::State;

pub mod compositor;
pub mod xdg_shell;

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm
    }
}

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;

    type PointerFocus = WlSurface;

    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
        &mut self.seats
    }
}

impl BufferHandler for State {
    fn buffer_destroyed(
        &mut self,
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
        // do nothing for now i guess?
    }
}

#[derive(Debug, Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {}

impl OutputHandler for State {}

impl SelectionHandler for State {
    type SelectionUserData = ();
}

impl DataDeviceHandler for State {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device
    }
}

impl DndGrabHandler for State {}
impl WaylandDndGrabHandler for State {
    fn dnd_requested<S: smithay::input::dnd::Source>(
        &mut self,
        source: S,
        icon: Option<WlSurface>,
        seat: smithay::input::Seat<Self>,
        serial: smithay::utils::Serial,
        type_: smithay::input::dnd::GrabType,
    ) {
        match type_ {
            GrabType::Pointer => {
                let ptr = seat.get_pointer().unwrap();
                let start_data = ptr.grab_start_data().unwrap();
        
                // create a dnd grab to start the operation
                let grab = DnDGrab::new_pointer(&self.display, start_data, source, seat);
                ptr.set_grab(self, grab, serial, Focus::Keep);
            }
            GrabType::Touch => {
                // no touch handling
                source.cancel();
            }
        }
    }
}

smithay::delegate_dispatch2!(State);
