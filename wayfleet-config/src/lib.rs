use std::{fs, path::Path};

use knus::{Decode, DecodeScalar, traits::ErrorSpan};
use miette::{NamedSource, SourceSpan};

use crate::{error::ConfigError, map::MapSpanned, padding::Padding, size::{Size, SizeRepr, Spaces}};

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

        let ConfigSpanned { map, privileged }: ConfigSpanned = knus::parse("config.kdl", &string).map_err(ConfigError::Parse)?;
        let src = NamedSource::new("config.kdl", string).with_language("KDL");
        
        let map = map.into_map(src)?;
        Ok(Config { 
            // screen, 
            map, 
            privileged 
        })
    }
}

#[derive(Debug, Decode)]
struct ConfigSpanned {
    // pub screen: Screen,
    #[knus(child)]
    pub map: MapSpanned,
    #[knus(child)]
    pub privileged: Privileged,
}

#[derive(Debug, Decode)]
pub struct Privileged {
    #[knus(child)]
    pub size: Size<SizeRepr>,
    
    #[knus(child, default)]
    pub spaces: Size<Spaces>,
    
    #[knus(child, default)]
    pub padding: Padding,
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    span: SourceSpan,
    value: T,
}

impl<T> Spanned<T> {
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

impl<T> AsRef<T> for Spanned<T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<S: ErrorSpan,T: Decode<S>> Decode<S> for Spanned<T> {
    fn decode_node(node: &knus::ast::SpannedNode<S>, ctx: &mut knus::decode::Context<S>) -> Result<Self, knus::errors::DecodeError<S>> {
        let span = node.span().clone().into();

        Ok(Self {
            span,
            value: T::decode_node(node, ctx)?
        })
    }
} 

impl<S: ErrorSpan, T: DecodeScalar<S>> DecodeScalar<S> for Spanned<T> {
    fn type_check(type_name: &Option<knus::span::Spanned<knus::ast::TypeName, S>>, ctx: &mut knus::decode::Context<S>) {
        T::type_check(type_name, ctx);
    }

    fn raw_decode(
        value: &knus::span::Spanned<knus::ast::Literal, S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        let span = value.span().clone().into();

        Ok(Self {
            span,
            value: T::raw_decode(value, ctx)?
        }) 
    }
}
// #[derive(Debug, )]
// pub struct Screen {
//     #[serde(default)]
//     pub size: SizeSpanned<SizeRepr>,
// }

// #[test]
// fn test() -> Result<(), ()>{
//     let x = String::from_utf8_lossy(&fs::read("config.toml").unwrap()).to_string().to_owned();

//     let config: ConfigRecv = toml::from_str(&x).map_err(|x| {
//         println!("{}", x);
//     })?;

//     println!("{config:#?}");
//     Ok(())
// }