use smithay::{backend::renderer::utils::on_commit_buffer_handler, desktop::{Space, Window}, input::{SeatHandler, SeatState}, reexports::{calloop::{EventLoop, LoopHandle}, wayland_server::{Display, DisplayHandle, backend::ClientData, protocol::wl_surface::WlSurface}}, utils::Size, wayland::{buffer::BufferHandler, compositor::{CompositorClientState, CompositorHandler, CompositorState, get_parent, is_sync_subsurface}, shell::xdg::{XdgShellHandler, XdgShellState}, shm::{ShmHandler, ShmState}}};

use crate::layout::{LayoutController, LayoutSettings};

pub struct State {
    loop_handle: LoopHandle<'static, Self>,
    display: DisplayHandle,
    layout: LayoutController,

    // smithay state
    compositor: CompositorState,
    shm: ShmState,
    xdg_shell: XdgShellState,
    seats: SeatState<Self>,
    space: Space<Window>,
}

impl State {
    pub fn new(event_loop: &EventLoop<'static, Self>, display: &Display<Self>, settings: LayoutSettings) -> Self {
        let loop_handle = event_loop.handle();
        let display = display.handle();

        Self {
            loop_handle,
            layout: LayoutController::new(settings),
            compositor: CompositorState::new::<Self>(&display),
            shm: ShmState::new::<Self>(&display, vec![]),
            xdg_shell: XdgShellState::new::<Self>(&display),
            space: Space::default(),
            seats: SeatState::<Self>::new(),
            display,
        }
    }
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut smithay::wayland::compositor::CompositorState {
        &mut self.compositor
    }

    fn client_compositor_state<'a>(&self, client: &'a smithay::reexports::wayland_server::Client) -> &'a smithay::wayland::compositor::CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
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

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell
    }

    fn new_toplevel(&mut self, surface: smithay::wayland::shell::xdg::ToplevelSurface) {
        let map = &mut self.layout.map;
        
        surface.with_pending_state(|state| {
            state.size = Some(map.get_size())
        });
        surface.send_pending_configure();

        let window = Window::new_wayland_window(surface);
        let Some(coord) = map.insert(window.clone()) else { todo!() } ;
        let pos = map.get_position(coord);
        self.space.map_element(window, pos, true);
    }

    fn new_popup(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface, positioner: smithay::wayland::shell::xdg::PositionerState) {
        todo!()
    }

    fn grab(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface, seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat, serial: smithay::utils::Serial) {
        todo!()
    }

    fn reposition_request(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface, positioner: smithay::wayland::shell::xdg::PositionerState, token: u32) {
        todo!()
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
    fn buffer_destroyed(&mut self, _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer) {
        
    }
}

pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {}

smithay::delegate_dispatch2!(State);