#![deny(deprecated)]
#![warn(clippy::todo)]

use std::{ffi::OsStr, process::{Command, Stdio}};

use crate::state::CONFIG;

pub mod state;
pub mod layout;
pub mod winit;
pub mod input;
pub mod handlers;
pub mod animations;

pub fn startup_spawns(socket: &OsStr, xwayland_display_number: &Option<String>) {
    let config = CONFIG.get().unwrap().as_ref();

    let startup = &config.startup;

    let spawn = |command: &mut Command| {
        let cmd = command
            .env("WAYLAND_DISPLAY", socket)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        
        if let Some(disp_n) = &xwayland_display_number {
            cmd
                .env("DISPLAY", disp_n);
        }

        let _ = cmd.spawn();
    };

    for (command, args) in startup.startup_spawn.iter().map(|x| x.split_first().unwrap()) {
        spawn(Command::new(command).args(args));
    }

    for spawn_sh in &startup.startup_spawn_sh {
        spawn(Command::new("sh").arg("-c").arg(spawn_sh));
    }
}