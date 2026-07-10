use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct Margins {
    #[serde(default)]
    pub left: u32,
    
    #[serde(default)]
    pub right: u32,
    
    #[serde(default)]
    pub top: u32,
    
    #[serde(default)]
    pub down: u32
}