use smithay::{desktop::{PopupKind, Window}, reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode, wayland::shell::xdg::{XdgShellHandler, XdgShellState, decoration::XdgDecorationHandler}};

use crate::state::State;

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
        let old_window  = match &self.layout.focus {
            crate::layout::controller::Focus::None => None,
            crate::layout::controller::Focus::Map(window) => Some(window.clone()),
            crate::layout::controller::Focus::Privileged(window) => Some(window.clone())  ,
        };
        // dbg!(&window);
        self.layout.insert_by_focus(window.clone());
        
        if let Some(old) = old_window {
            self.refocus(&old, &window);
        } else {
            self.set_kb_focus(&window);
        }
    }

    fn new_popup(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
    ) {
        let geometry = positioner.get_geometry();
        surface.with_pending_state(|state| {
            state.geometry = geometry;
        });
        surface.send_configure().unwrap();
        
        self.popups.track_popup(PopupKind::Xdg(surface)).unwrap();
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