use smithay::{desktop::{LayerSurface, layer_map_for_output}, output::Output, wayland::shell::wlr_layer::WlrLayerShellHandler};

use crate::state::State;

impl WlrLayerShellHandler for State {
    fn shell_state(&mut self) -> &mut smithay::wayland::shell::wlr_layer::WlrLayerShellState {
        &mut self.layer_state
    }

    fn new_layer_surface(
        &mut self,
        surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
        _layer: smithay::wayland::shell::wlr_layer::Layer,
        namespace: String,
    ) {
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .unwrap_or_else(|| self.layout.space.outputs().next().unwrap().clone());
        let mut map = layer_map_for_output(&output);
        let layer = LayerSurface::new(surface, namespace);
        layer.layer_surface().send_configure();
        map.map_layer(&layer).unwrap();
        map.arrange();
        let available = map.non_exclusive_zone();
        self.layout.update_available_state(available);
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {
        
        let mut outputs = self.layout.space.outputs();

        let Some((mut map, layer)) = outputs.find_map(|o| {
            let map = layer_map_for_output(o);
            let layer = map
                .layers()
                .find(|&layer| layer.layer_surface() == &surface)
                .cloned();
            layer.map(|layer| (map, layer))
        }) else {
            return;
        };

        map.unmap_layer(&layer);
        let available = map.non_exclusive_zone();
        drop(map);
        drop(outputs);
        self.layout.update_available_state(available);
    }

    fn ack_configure(&mut self, _surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface, _configure: smithay::wayland::shell::wlr_layer::LayerSurfaceConfigure) {
        let Some(mut map) = self.layout.space.outputs().map(|o| {
            layer_map_for_output(o)
        }).next()  else { // TODO: this next is fine because we ever only allow one output, but i know it'll shot me in the foot some time in the future
            return;
        };
        map.arrange();
        let available = map.non_exclusive_zone();
        drop(map);
        self.layout.update_available_state(available);
    }
}