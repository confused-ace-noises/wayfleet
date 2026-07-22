use std::{cell::RefCell, fs, path::Path, rc::Rc};

use knus::{Decode, DecodeScalar, span::Span, traits::ErrorSpan};
use miette::{NamedSource, SourceSpan};

use crate::{amount::Amount, error::ConfigError, input::Input, keybinds::KeyBinds, map::MapSpanned, padding::Padding, size::{Size, Spaces}};

pub mod size;
pub mod amount;
pub mod padding;
pub mod map;
pub mod error;
pub mod keybinds;
pub mod input;

pub use map::Map;

#[derive(Debug, Decode)]
pub struct Config {
    #[knus(child)]
    pub input: Input,

    // pub screen: Screen,
    #[knus(child)]
    pub layout: Layout,

    #[knus(child)]
    pub keybinds: KeyBinds
}

impl Config {
    pub fn parse<P: AsRef<Path>>(file: P) -> Result<Self, ConfigError> {
        let read = fs::read(file)?;

        let string = String::from_utf8_lossy(&read).to_string();

        let src = NamedSource::new("config.kdl", string.clone()).with_language("KDL");
        let cell = Rc::new(RefCell::new(Vec::<ConfigError>::new()));
        let config: Config = knus::parse_with_context::<_, Span, _>("config.kdl", &string, |ctx| {
            ctx.set(src);
            ctx.set(cell.clone());
        }).map_err(ConfigError::Parse)?;

        let mut vec = Rc::try_unwrap(cell).unwrap().into_inner();
        if !vec.is_empty() {
            let x = vec.remove(0);
            return Err(x);
        };

        Ok(config)
    }
}

#[derive(Debug)]
pub struct Layout {
    pub map: Map,
    pub privileged: Privileged,
}

impl LayoutSpanned {
    pub fn into_layout(self, src: &NamedSource<String>) -> Result<Layout, ConfigError> {
        let LayoutSpanned { map, privileged } = self;

        let map = map.into_map(src)?;

        Ok(Layout { map, privileged })
    }
}

impl<S: ErrorSpan> Decode<S> for Layout {
    fn decode_node(node: &knus::ast::SpannedNode<S>, ctx: &mut knus::decode::Context<S>) -> Result<Self, knus::errors::DecodeError<S>> {
        let spanned = LayoutSpanned::decode_node(node, ctx)?;

        let src = ctx.get::<NamedSource<String>>().unwrap();
        
        let layout = spanned.into_layout(src);

        match layout {
            Ok(layout) => Ok(layout),
            Err(config_err) => {
                // HACK: couldn't find any way to carry rich miette info out of this function,
                // so the error is pushed to a vector through the context, and the toplevel parser is
                // responsible for 
                let errors: &Rc<RefCell<Vec<ConfigError>>> = ctx.get::<Rc<RefCell<Vec<ConfigError>>>>().unwrap();
                let mut borrow = errors.borrow_mut();

                borrow.push(config_err);
                
                drop(borrow);

                let dummy_layout = Layout {
                    map: Map { size: Default::default(), cells: Default::default(), spaces: Default::default(), margins: Default::default() },
                    privileged: Privileged::dummy(),
                };

                Ok(dummy_layout)
            },
        }
    }
}


#[derive(Debug, Decode)]
struct LayoutSpanned {
    // pub screen: Screen,
    #[knus(child)]
    pub map: MapSpanned,
    #[knus(child)]
    pub privileged: Privileged,
}

#[derive(Debug, Decode)]
pub struct Privileged {
    #[knus(child, unwrap(argument))]
    pub height: Amount,

    #[knus(child, unwrap(argument), default)]
    pub standard_width: Amount,
    
    #[knus(child, default)]
    pub spaces: Size<Spaces>,
    
    #[knus(child, default)]
    pub padding: Padding,
}

impl Privileged {
    pub fn dummy() -> Self {
        Self {
            height: Default::default(),
            spaces: Default::default(),
            standard_width: Default::default(),
            padding: Default::default()
        }
    }
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