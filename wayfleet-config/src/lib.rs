use std::{fs, path::Path};

use miette::NamedSource;
use serde::{self, Deserialize};

use crate::{error::ConfigError, map::MapRecv, padding::Margins, size::{SizeRepr, Size, Spaces}};

pub mod size;
pub mod amount;
pub mod padding;
pub mod map;
pub mod error;

pub use map::Map;

pub struct Config {
    // pub screen: Screen,
    pub map: Map,
    pub privileged: Privileged,
}

impl Config {
    pub fn parse<P: AsRef<Path>>(file: P) -> Result<Self, ConfigError> {
        let read = fs::read(file)?;

        let string = String::from_utf8_lossy(&read).to_string();

        let src = NamedSource::new("config.toml", string).with_language("TOML");
        let ConfigRecv { map, privileged }: ConfigRecv = toml::de::from_str(src.inner()).map_err(|e| ConfigError::from_serde(e, src.clone()))?;
        
        let map = map.into_map(src)?;
        Ok(Config { 
            // screen, 
            map, 
            privileged 
        })
    }
}

#[derive(Debug, Deserialize)]
struct ConfigRecv {
    // pub screen: Screen,
    pub map: MapRecv,
    pub privileged: Privileged,
}

#[derive(Debug, Deserialize)]
pub struct Privileged {
    pub size: Size<SizeRepr>,

    #[serde(default)]
    pub spaces: Size<Spaces>,
    
    #[serde(default)]
    pub margins: Margins,
}

#[derive(Debug, Deserialize)]
pub struct Screen {
    #[serde(default)]
    pub size: Size<SizeRepr>,
}

#[test]
fn test() -> Result<(), ()>{
    let x = String::from_utf8_lossy(&fs::read("config.toml").unwrap()).to_string().to_owned();

    let config: ConfigRecv = toml::from_str(&x).map_err(|x| {
        println!("{}", x);
    })?;

    println!("{config:#?}");
    Ok(())
}