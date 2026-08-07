use std::{sync::OnceLock, time::Duration};

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::surface::WaylandSurfaceRenderElement,
            gles::{GlesPixelProgram, GlesRenderer, UniformName, UniformType},
        },
        winit::{self, WinitEvent},
    }, desktop::layer_map_for_output, output::{Mode, Output, PhysicalProperties, Scale, Subpixel}, reexports::{
        calloop::EventLoop,
        wayland_server::Display,
    }, utils::Transform,
};
use wayfleet_config::Config;

use crate::state::{BackendData, OutputState, State, Winit};

pub static BORDER_SHADER: OnceLock<GlesPixelProgram> = OnceLock::new();

pub fn init_winit(
    event_loop: &mut EventLoop<'static, State<Winit>>,
    display: Display<State<Winit>>,
    config: Config,
) -> Result<State<Winit>, Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init::<GlesRenderer>()?;

    let shader = backend
        .renderer()
        .compile_custom_pixel_shader(
            include_str!("shaders/border.glsl"),
            &[
                UniformName::new("border_color", UniformType::_4f),
                UniformName::new("border_thickness", UniformType::_1f),
                UniformName::new("corner_radius", UniformType::_1f),
            ],
        )
        .inspect_err(|x| eprintln!("{x}"))
        .unwrap();

    BORDER_SHADER.set(shader).unwrap();

    let size = backend.window_size();
    let scale_factor = backend.scale_factor();

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
    };

    // OutputManager

    let mut state = State::<Winit>::new(event_loop, display, config, output_state);

    let _global = output.create_global::<State<Winit>>(&state.display);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        Some(Scale::Fractional(scale_factor)),
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    state.backend_data.layout_controller_mut().space.map_output(&output, (0, 0));
    state.xwayland_override_redirects_space.map_output(&output, (0, 0));
    // let x = ZxdgOutputManagerV1::from(value);

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| {
            match event {
                WinitEvent::Resized { size, scale_factor } => {
                    output.change_current_state(
                        Some(Mode {
                            size,
                            refresh: 60_000,
                        }),
                        None,
                        None,
                        None,
                    );

                    let output_state = OutputState {
                        size,
                        scale_factor,
                    };

                    state.resize_output(output_state);
                }
                WinitEvent::Input(event) => state.run_input(event),
                WinitEvent::Redraw => {
                    // let layer_map = layer_map_for_output(&output);

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
                            [
                                &state.xwayland_override_redirects_space,
                                &state.backend_data.layout_controller().space, 
                            ],
                            &[],
                            &mut damage_tracker,
                            [0.88, 0.69, 1.0, 1.0], // mauve
                        )
                        .unwrap()
                    };

                    backend
                        .submit(render_result.damage.map(|x| x.as_slice()))
                        .unwrap();

                    state.backend_data.layout_controller().space.elements().chain(state.xwayland_override_redirects_space.elements()).for_each(|window| {
                        window.send_frame(
                            &output,
                            state.start_time.elapsed(),
                            Some(Duration::ZERO),
                            |_, _| Some(output.clone()),
                        )
                    });

                    let map = layer_map_for_output(&output);
                    for layer in map.layers() {
                        layer.send_frame(
                            &output,
                            state.start_time.elapsed(),
                            Some(Duration::ZERO),
                            |_, _| Some(output.clone()),
                        );
                    }

                    state.backend_data.layout_controller_mut().space.refresh();
                    state.popups.cleanup();
                    let _ = state.display.flush_clients();

                    // Ask for redraw to schedule new frame.
                    backend.window().request_redraw();
                }
                WinitEvent::CloseRequested => {
                    state.loop_signal.stop();
                }

                WinitEvent::Focus(_) => {
                    // ? NOTE: due to a known bug in the `winit` implementation
                    // ? of keyboard state modifiers, if the window of our nested
                    // ? compositor (running with the winit backend) goes from a focused to
                    // ? an unfocused state, and while in the unfocused state the state of a mod
                    // ? key changes, this change WILL NOT be registered and we'll end up
                    // ? with an unsync'd ModifiersState. This also means, for example,
                    // ? that if we're using a compositor that uses a Super+<Direction> keybind
                    // ? to change the focus, the Super key will be pressed while in our compositor's
                    // ? window, and then will be released once our compositor's window loses focus.
                    // ? Now the states are unsync'd, but if the compositor's window is re-focused
                    // ? while pressing Super, this will then depress in our window and fix the
                    // ? broken state. If this doesn't happen, because, for example, the window is
                    // ? re-focused by clicking on it, the Super key will be stuck in a pressed
                    // ? state and everything will be completely off. I have no idea of how to fix
                    // ? this, for the time being, it'll be broken.
                    // ? refer to: https://github.com/Smithay/smithay/issues/1353
                }
            };
        })?;

    Ok(state)
}
