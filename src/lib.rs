#![deny(deprecated)]
#![warn(clippy::todo)]

use std::{ffi::OsStr, mem::MaybeUninit, ops::{Deref, DerefMut}, process::{Command, Stdio}};

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

pub struct Late<T>{
    maybe_uninit: MaybeUninit<T>,
    has_init: bool
}

impl<T> Default for Late<T> {
    fn default() -> Self {
        Self::uninit()
    }
}

impl<T> Late<T> {
    pub const fn uninit() -> Self {
        Late {
            maybe_uninit: MaybeUninit::uninit(),
            has_init: false,
        }
    }

    pub fn init(&mut self, val: T) {
        if self.is_init() {
            unsafe { self.maybe_uninit.assume_init_drop() };
            self.maybe_uninit.write(val);
        } else {
            self.maybe_uninit.write(val);
            self.has_init = true;
        }
    }

    pub fn is_init(&self) -> bool {
        self.has_init
    }  
}

impl<T> Deref for Late<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: caller guarantees init
        unsafe { self.maybe_uninit.assume_init_ref() }
    }
}

impl<T> DerefMut for Late<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.maybe_uninit.assume_init_mut() }
    }
}