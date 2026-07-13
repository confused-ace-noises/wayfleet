use knus::Decode;

#[derive(Debug, Default, Decode)]
pub struct Padding {
    #[knus(child, unwrap(argument), default)]
    pub left: i32,

    #[knus(child, unwrap(argument), default)]
    pub right: i32,

    #[knus(child, unwrap(argument), default)]
    pub top: i32,

    #[knus(child, unwrap(argument), default)]
    pub down: i32,
}