use std::{ffi::OsString, sync::Arc, time::Instant};

use smithay::{
    desktop::{PopupManager, Window}, input::{Seat, SeatState}, reexports::{
    calloop::{self, EventLoop, Interest, LoopHandle, LoopSignal, generic::Generic},
        wayland_server::{
            Display, DisplayHandle,
        },
    }, utils::{Logical, Physical, SERIAL_COUNTER, Scale, Size}, wayland::{
        compositor::CompositorState, seat::WaylandFocus, selection::data_device::DataDeviceState, shell::xdg::{
            XdgShellState,
            decoration::XdgDecorationState,
        }, shm::ShmState, socket::ListeningSocketSource,
    },
};
use wayfleet_config::Config;

use crate::{handlers::ClientState, layout::controller::LayoutController};

pub struct State {
    pub start_time: Instant,
    pub loop_handle: LoopHandle<'static, Self>,
    pub loop_signal: LoopSignal,
    pub display: DisplayHandle,
    pub layout: LayoutController,
    pub socket: OsString,
    pub output_state: OutputState,

    // smithay state
    pub compositor: CompositorState,
    pub shm: ShmState,
    pub xdg_shell: XdgShellState,
    pub seats: SeatState<Self>,
    pub seat: Seat<Self>,
    pub decorations: XdgDecorationState,
    pub popups: PopupManager,
    pub data_device: DataDeviceState,
}

impl State {
    pub fn new(
        event_loop: &mut EventLoop<'static, Self>,
        display_real: Display<Self>,
        config: Config,
        output_state: OutputState,
    ) -> Self {
        let start_time = Instant::now();
        let loop_signal = event_loop.get_signal();
        let loop_handle = event_loop.handle();
        let display = display_real.handle();

        let socket = ListeningSocketSource::new_auto().unwrap();

        let socket_name = socket.socket_name().to_os_string();

        loop_handle
            .insert_source(socket, move |stream, _, state: &mut State| {
                state
                    .display
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        loop_handle
            .insert_source(
                Generic::new(display_real, Interest::READ, calloop::Mode::Level),
                |_, display_io, state| {
                    unsafe {
                        display_io.get_mut().dispatch_clients(state).unwrap();
                    }
                    Ok(calloop::PostAction::Continue)
                },
            )
            .unwrap();

        let mut seats = SeatState::<Self>::new();
        let seat = seats.new_wl_seat(&display, "winit");

        Self {
            loop_signal,
            start_time,
            loop_handle,
            layout: LayoutController::new(config, &output_state),
            compositor: CompositorState::new::<Self>(&display),
            shm: ShmState::new::<Self>(&display, vec![]),
            xdg_shell: XdgShellState::new::<Self>(&display),
            data_device: DataDeviceState::new::<Self>(&display),
            seats,
            decorations: XdgDecorationState::new::<Self>(&display),
            display,
            socket: socket_name,
            output_state,
            seat,
            popups: PopupManager::default(),
        }
    }

    pub fn set_kb_focus(&mut self, window: &Window) {
        if let Some(x) = self.seat.get_keyboard() {
            x.set_focus(
                self,
                window.wl_surface().map(|x| x.into_owned()),
                SERIAL_COUNTER.next_serial(),
            );
        }
    }
}

#[derive(Debug)]
pub struct OutputState {
    pub size: Size<i32, Physical>,
    pub scale_factor: i32,
    pub changed: bool,
}

impl OutputState {
    pub fn logical_size(&self) -> Size<i32, Logical> {
        self.size.to_logical(Scale::from(self.scale_factor))
    }
}