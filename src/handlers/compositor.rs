use smithay::{
    backend::renderer::utils::on_commit_buffer_handler, wayland::{
        compositor::{CompositorHandler, get_parent, is_sync_subsurface},
        seat::WaylandFocus,
    }, xwayland::XWaylandClientData,
};

use crate::{handlers::ClientState, layout::controller::LayoutController, state::State};

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut smithay::wayland::compositor::CompositorState {
        &mut self.compositor
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a smithay::wayland::compositor::CompositorClientState {
        if let Some(state) = client.get_data::<ClientState>() {
            &state.compositor_state
        } else if let Some(state) = client.get_data::<XWaylandClientData>() {
            &state.compositor_state
        } else {
            unimplemented!("what does this even want")
        }
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
                .find(|w| w.toplevel().map(|x| *x.wl_surface() == root).unwrap_or(false))
            {
                window.on_commit();
            }
        };

        self.popups.commit(surface);
    }

    fn destroyed(
        &mut self,
        _surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        let window = self
            .layout
            .space
            .elements()
            .find(|x| x.wl_surface().is_some_and(|surface| *surface.as_ref() == *_surface))
            .cloned();
        
        if let Some(window) = window{
            LayoutController::remove(self, &window);
        }
    }
}
