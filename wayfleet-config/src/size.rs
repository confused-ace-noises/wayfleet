use knus::{Decode, DecodeScalar, errors::DecodeError, traits::ErrorSpan};

use crate::{Spanned, amount::Amount};

#[derive(Debug, Decode)]
pub struct Grid {
    #[knus(child, unwrap(argument))]
    pub rows: Amount,
    
    #[knus(child, unwrap(argument))]
    pub columns: Amount,
}

impl From<GridRecv> for Grid {
    fn from(value: GridRecv) -> Self {
        Self {
            rows: value.rows.value,
            columns: value.columns.value,
        }
    }
}

#[derive(Debug, Decode)]
pub struct GridRecv {
    #[knus(child, unwrap(argument))]
    pub rows: Spanned<Amount>,
    
    #[knus(child, unwrap(argument))]
    pub columns: Spanned<Amount>,
}

#[derive(Debug, Decode)]
pub struct SizeRepr {
    #[knus(child, unwrap(argument))]
    pub height: Amount, 

    #[knus(child, unwrap(argument))]
    pub width: Amount
}

impl From<SizeReprRecv> for SizeRepr {
    fn from(value: SizeReprRecv) -> Self {
        Self {
            height: value.height.value,
            width: value.width.value,
        }
    }
}

#[derive(Debug, Decode)]
pub struct SizeReprRecv {
    #[knus(child, unwrap(argument))]
    pub height: Spanned<Amount>, 

    #[knus(child, unwrap(argument))]
    pub width: Spanned<Amount>
}

#[derive(Debug, Default, Copy, Clone, Decode)]
pub struct Spaces {
    #[knus(child, unwrap(argument))]
    pub horizontal: u32,

    #[knus(child, unwrap(argument))]
    pub vertical: u32,
}

#[derive(Debug, Default)]
pub enum Size<T> {
    Specified(T),

    #[default]
    Auto,
}

impl<S: ErrorSpan, T: Decode<S>> Decode<S> for Size<T> {
    fn decode_node(node: &knus::ast::SpannedNode<S>, ctx: &mut knus::decode::Context<S>) -> Result<Self, DecodeError<S>> {
        if let Some(arg) = node.arguments.first() {
            if node.arguments.len() > 1 {
                ctx.emit_error(DecodeError::unexpected(
                    node,
                    "argument",
                    "expected only a single `auto` argument",
                ));
            }

            if node.children().next().is_some() {
                ctx.emit_error(DecodeError::unexpected(
                    node,
                    "node",
                    "cannot combine `auto` with children",
                ));
            }

            let word: String = DecodeScalar::decode(arg, ctx)?;
            if word != "auto" {
                ctx.emit_error(DecodeError::unexpected(
                    node,
                    "value",
                    format!("expected `auto`, found `{word}`"),
                ));
            }

            return Ok(Size::Auto);
        }

        T::decode_node(node, ctx).map(Size::Specified)
    }
}


impl<T> Size<T> {
    pub fn into<U: From<T>>(self) -> Size<U> {
        match self {
            Size::Specified(t) => Size::Specified(t.into()),
            Size::Auto => Size::Auto,
        }
    }

    pub fn unwrap_or_else(self, f: impl FnOnce() -> T) -> T {
        match self {
            Size::Specified(t) => t,
            Size::Auto => f(),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            Size::Specified(t) => t,
            Size::Auto => unimplemented!(),
        }
    }

    pub fn unwrap_ref(&self) -> &T {
        match self {
            Size::Specified(t) => t,
            Size::Auto => unimplemented!(),
        }
    }
}