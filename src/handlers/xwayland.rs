use std::process::Stdio;

use smithay::{desktop::Window, reexports::{calloop::LoopHandle, wayland_server::DisplayHandle}, wayland::xwayland_shell::XWaylandShellHandler, xwayland::{X11Wm, XWayland, XWaylandEvent, XwmHandler}};

use crate::{layout::{WayfleetWindow, controller::LayoutController}, state::State};

pub fn create_xwayland(handle: LoopHandle<'static, State>, display_handle: DisplayHandle) {
    let (xwayland, client) = XWayland::spawn::<
        _,
        _,
        [(String, String); 0],
        _,
        &[String],
        _,
    >(
        &display_handle,
        None,
        [],
        &[],
        true,
        Stdio::null(),
        Stdio::null(),
        |_| (),
    ).expect("failed to start xwayland");

    let colned = handle.clone();

    handle.insert_source(xwayland, move |event, _, data| match event {
        XWaylandEvent::Ready { x11_socket, display_number } => {
            let wm = X11Wm::start_wm(
                colned.clone(),
                &display_handle,
                x11_socket,
                client.clone(),
            ).expect("Failed to attach X11 Window Manager");

            data.xwalyand_manager = Some(wm);
            data.xwayland_display_number = Some(format!(":{}", display_number));

            unsafe { std::env::set_var("DISPLAY", format!(":{}", display_number)) }
        }
        XWaylandEvent::Error => eprintln!("XWayland failed to start!"),
    }).expect("failed to insert xwayland source");
}

impl XwmHandler for State {
    fn xwm_state(&mut self, _: smithay::xwayland::xwm::XwmId) -> &mut X11Wm {
        self.xwalyand_manager.as_mut().expect("what should i even do here")
    }

    fn new_window(&mut self, _xwm: smithay::xwayland::xwm::XwmId, _window: smithay::xwayland::X11Surface) {
        // do nothing ig?
    }

    fn new_override_redirect_window(&mut self, _xwm: smithay::xwayland::xwm::XwmId, _window: smithay::xwayland::X11Surface) {
        // do nothing ig?
    }

    fn map_window_request(&mut self, _xwm: smithay::xwayland::xwm::XwmId, window: smithay::xwayland::X11Surface) {
        window.set_mapped(true).unwrap();
        let window = Window::new_x11_window(window);

        let old_window  = match &self.layout.focus {
            crate::layout::controller::Focus::None => None,
            crate::layout::controller::Focus::Map(window) => Some(window.clone()),
            crate::layout::controller::Focus::Privileged(window) => Some(window.clone())  ,
        };

        self.layout.insert_by_focus_w_forcing(window.clone());
        
        
        let bbox = self.layout.space.element_bbox(&WayfleetWindow::dummy(window.clone())).unwrap();
        let Some(xsurface) = window.x11_surface() else {
            unreachable!()
        };
        xsurface.configure(Some(bbox)).unwrap();

        if let Some(old) = old_window {
            self.refocus(&old, &window);
        } else {
            self.set_kb_focus(&window);
        }
    }

    fn mapped_override_redirect_window(&mut self, _xwm: smithay::xwayland::xwm::XwmId, window: smithay::xwayland::X11Surface) {
        let location = window.last_configure().loc;
        let window = Window::new_x11_window(window);
        self.xwayland_override_redirects_space.map_element(WayfleetWindow::new_x11_OR(window), location, true);
    }

    fn unmapped_window(&mut self, _xwm: smithay::xwayland::xwm::XwmId, window: smithay::xwayland::X11Surface) {
        if window.is_override_redirect() {
            let window = self.xwayland_override_redirects_space.elements().find(|e| matches!(e.x11_surface(), Some(w) if w == &window)).cloned();
            if let Some(yes_window) = window {
                self.xwayland_override_redirects_space.unmap_elem(&yes_window);
            }
        } else {
            let wayfleet_window = self.layout.space.elements().find(|e| matches!(e.x11_surface(), Some(w) if w == &window)).cloned();
            if let Some(yes_window) = wayfleet_window {
                LayoutController::remove(self, &yes_window);
            }

            window.set_mapped(false).unwrap();
        }
    }

    fn destroyed_window(&mut self, xwm: smithay::xwayland::xwm::XwmId, window: smithay::xwayland::X11Surface) {
        Self::unmapped_window(self, xwm, window);
    }

    fn configure_request(
        &mut self,
        _xwm: smithay::xwayland::xwm::XwmId,
        _window: smithay::xwayland::X11Surface,
        _x: Option<i32>,
        _y: Option<i32>,
        _w: Option<u32>,
        _h: Option<u32>,
        _reorder: Option<smithay::xwayland::xwm::Reorder>,
    ) {
        // x11 is so entitled smh (/hj)
    }

    fn configure_notify(
        &mut self,
        _xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        geometry: smithay::utils::Rectangle<i32, smithay::utils::Logical>,
        _above: Option<smithay::xwayland::xwm::X11Window>,
    ) {
        if window.is_override_redirect() {
            let Some(desktop_window) = self
                .xwayland_override_redirects_space
                .elements()
                .find(|e| matches!(e.x11_surface(), Some(w) if w == &window))
                .cloned() else
            {
                return;
            };

            self.xwayland_override_redirects_space
                .map_element(desktop_window, geometry.loc, true); // TODO: should this be activated or not? 
        }
    }

    fn resize_request(&mut self, _xwm: smithay::xwayland::xwm::XwmId, _window: smithay::xwayland::X11Surface, _button: u32, _resize_edge: smithay::xwayland::xwm::ResizeEdge) {
        // again, x11 is so entitled
    }

    fn move_request(&mut self, _xwm: smithay::xwayland::xwm::XwmId, _window: smithay::xwayland::X11Surface, _button: u32) {
        // again-again, x11 is so entitledddd
    }
}

impl XWaylandShellHandler for State {
    fn xwayland_shell_state(&mut self) -> &mut smithay::wayland::xwayland_shell::XWaylandShellState {
        &mut self.xwayland_shell
    }
}