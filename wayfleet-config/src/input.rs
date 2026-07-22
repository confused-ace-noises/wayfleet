use knus::Decode;

#[derive(Debug, Decode)]
pub struct Input {
    #[knus(child)]
    pub keyboard: Keyboard,
}

#[derive(Debug, Decode)]
pub struct Keyboard {
    #[knus(child, unwrap(argument))]
    pub layout: String,
}