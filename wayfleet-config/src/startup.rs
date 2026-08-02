use knus::Decode;

#[derive(Debug, Decode, Default)]
pub struct Startup {
    #[knus(children(name = "spawn"), unwrap(arguments))]
    pub startup_spawn: Vec<Vec<String>>,

    #[knus(children(name = "spawn-sh"), unwrap(argument))]
    pub startup_spawn_sh: Vec<String>,
}