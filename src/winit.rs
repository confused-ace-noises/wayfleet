use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement, gles::GlesRenderer,
        }, winit::{self, WinitEvent},
    }, output::{Mode, Output, PhysicalProperties, Subpixel}, reexports::{calloop::EventLoop, wayland_server::Display}, utils::Transform,
};
use wayfleet_config::Config;

use crate::state::{OutputState, State};

pub fn init_winit(
    event_loop: &mut EventLoop<'static, State>,
    display: Display<State>,
    config: Config,
) -> Result<State, Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init()?;

    let size = backend.window_size();
    let scale_factor = backend.scale_factor() as i32;

    let mode = Mode {
        size,
        refresh: 60_000,
    };

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Smithay".into(),
            model: "Winit".into(),
            serial_number: "Unknown".into(),
        },
    );

    let output_state = OutputState {
        size,
        scale_factor,
        changed: false,
    };

    println!("{:?}", output_state);

    let mut state = State::new(event_loop, display, config, output_state);

    let _global = output.create_global::<State>(&state.display);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    state.layout.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| {
            match event {
                WinitEvent::Resized { size, .. } => {
                    

                    output.change_current_state(
                        Some(Mode {
                            size,
                            refresh: 60_000,
                        }),
                        None,
                        None,
                        None,
                    );
                }
                WinitEvent::Input(event) => state.run_input(event),
                WinitEvent::Redraw => {
                    let render_result = {
                        let (renderer, mut framebuffer) = backend.bind().unwrap();
                        smithay::desktop::space::render_output::<
                            _,
                            WaylandSurfaceRenderElement<GlesRenderer>,
                            _,
                            _,
                        >(
                            &output,
                            renderer,
                            &mut framebuffer,
                            1.0,
                            0,
                            [&state.layout.space],
                            &[],
                            &mut damage_tracker,
                            [0.88, 0.69, 1.0, 1.0], // mauve
                        )
                        .unwrap()
                    };

                    backend
                        .submit(render_result.damage.map(|x| x.as_slice()))
                        .unwrap();

                    state.layout.space.elements().for_each(|window| {
                        window.send_frame(
                            &output,
                            state.start_time.elapsed(),
                            Some(Duration::ZERO),
                            |_, _| Some(output.clone()),
                        )
                    });

                    state.layout.space.refresh();
                    state.popups.cleanup();
                    let _ = state.display.flush_clients();

                    // Ask for redraw to schedule new frame.
                    backend.window().request_redraw();
                }
                WinitEvent::CloseRequested => {
                    state.loop_signal.stop();
                }
                _ => (),
            };
        })?;

    Ok(state)
}
