use smithay::{desktop::{LayerSurface, WindowSurfaceType, layer_map_for_output}, output::Output, wayland::{compositor::add_post_commit_hook, shell::wlr_layer::WlrLayerShellHandler}};

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

        println!("about to focus layer");
        if layer.can_receive_keyboard_focus() {
            println!("focusing layer");
            self.focus_layer(&layer);
        }

        let cloned_layer = layer.clone();
        
        add_post_commit_hook::<State, _>(layer.clone().wl_surface(), move |state, _, _| {
            if cloned_layer.can_receive_keyboard_focus() {
                println!("focusing layer");
                state.focus_layer(&layer);
            }
        });

        map.arrange();
        let available = map.non_exclusive_zone();
        println!("{available:?}");
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
        self.defocus_layer();
        self.layout.update_available_state(available);
    }

    fn ack_configure(&mut self, surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface, _configure: smithay::wayland::shell::wlr_layer::LayerSurfaceConfigure) {
        let Some(mut map) = self.layout.space.outputs().map(|o| {
            layer_map_for_output(o)
        }).next()  else { // TODO: this next is fine because we ever only allow one output, but i know it'll shot me in the foot some time in the future
            return;
        };
        map.arrange();
        let available = map.non_exclusive_zone();
        println!("about to focus layer");
        if let Some(layer) =  map.layer_for_surface(&surface, WindowSurfaceType::empty()).cloned() && layer.can_receive_keyboard_focus() {
            println!("focusing layer");
            drop(map);    
            self.focus_layer(&layer);
        } else {
            drop(map);
        }
        self.layout.update_available_state(available);

    }
}