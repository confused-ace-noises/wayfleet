use std::{ffi::OsString, sync::Arc, time::Instant};

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    desktop::Window,
    input::{Seat, SeatHandler, SeatState},
    reexports::{
        calloop::{self, EventLoop, Interest, LoopHandle, LoopSignal, generic::Generic},
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
        wayland_server::{
            Display, DisplayHandle, backend::ClientData, protocol::wl_surface::WlSurface,
        },
    },
    utils::{Logical, SERIAL_COUNTER, Size},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorClientState, CompositorHandler, CompositorState, get_parent,
            is_sync_subsurface,
        },
        output::OutputHandler,
        seat::WaylandFocus,
        shell::xdg::{
            XdgShellHandler, XdgShellState,
            decoration::{XdgDecorationHandler, XdgDecorationState},
        },
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};

use crate::layout::{LayoutController, LayoutSettings};

pub struct State {
    pub start_time: Instant,
    pub loop_handle: LoopHandle<'static, Self>,
    pub loop_signal: LoopSignal,
    pub display: DisplayHandle,
    pub layout: LayoutController,
    pub socket: OsString,

    pub window_size: Size<i32, Logical>,

    // smithay state
    pub compositor: CompositorState,
    pub shm: ShmState,
    pub xdg_shell: XdgShellState,
    pub seats: SeatState<Self>,
    pub seat: Seat<Self>,
    pub decorations: XdgDecorationState,
    // pub space: Space<Window>,
}

impl State {
    pub fn new(
        event_loop: &mut EventLoop<'static, Self>,
        display_real: Display<Self>,
        settings: LayoutSettings,
        window_size: Size<i32, Logical>,
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
            layout: LayoutController::new(settings),
            compositor: CompositorState::new::<Self>(&display),
            shm: ShmState::new::<Self>(&display, vec![]),
            xdg_shell: XdgShellState::new::<Self>(&display),
            seats,
            decorations: XdgDecorationState::new::<Self>(&display),
            display,
            socket: socket_name,
            window_size,
            seat,
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

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut smithay::wayland::compositor::CompositorState {
        &mut self.compositor
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a smithay::wayland::compositor::CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(
        &mut self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        on_commit_buffer_handler::<Self>(surface);

        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .layout
                .space
                .elements()
                .find(|w| w.toplevel().unwrap().wl_surface() == &root)
            {
                window.on_commit();
            }
        };

        // xdg_shell::handle_commit(&mut self.popups, &self.space, surface);
        // resize_grab::handle_commit(&mut self.space, surface);
    }
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm
    }
}

#[allow(unused_variables)] // tmp
impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell
    }

    fn new_toplevel(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {
        let map = &mut self.layout.map;

        surface.with_pending_state(|state| {
            state.size = Some(map.get_size());
        });
        surface.send_configure();

        let window = Window::new_wayland_window(surface);
        dbg!(&window);
        self.layout.insert_generic(window.clone());
        self.set_kb_focus(&window);
    }

    fn new_popup(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
    ) {
        todo!()
    }

    fn grab(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
        todo!()
    }

    fn reposition_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
        token: u32,
    ) {
        todo!()
    }
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: smithay::wayland::shell::xdg::ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ServerSide);
        });
        toplevel.send_configure();
    }

    fn request_mode(
        &mut self,
        toplevel: smithay::wayland::shell::xdg::ToplevelSurface,
        _mode: Mode,
    ) {
        toplevel.with_pending_state(|state| {
            // just ignore the request :p
            state.decoration_mode = Some(Mode::ServerSide);
        });
        toplevel.send_configure();
    }

    fn unset_mode(&mut self, toplevel: smithay::wayland::shell::xdg::ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ServerSide);
        });
        toplevel.send_configure();
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

smithay::delegate_dispatch2!(State);
