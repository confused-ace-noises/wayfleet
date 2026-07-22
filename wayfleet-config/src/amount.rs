use std::ops::Deref;

use knus::{DecodeScalar, ast::Literal, errors::{DecodeError, ExpectedType}, traits::ErrorSpan};

use crate::amount::Amount::Auto;


#[derive(Debug, Clone, Copy, Default)]
pub enum Amount {
    #[default]
    Auto,
    Specified(i32),
}

impl Amount {
    pub fn unwrap(self) -> i32 {
        match self {
            Auto => unimplemented!(),
            Self::Specified(x) => x
        }
    }

    pub fn unwrap_or(self, or: i32) -> i32 {
        match self {
            Auto => or,
            Amount::Specified(amount) => amount,
        }
    }

    pub fn unwrap_or_else(self, f: impl FnOnce() -> i32) -> i32 {
        match self {
            Auto => f(),
            Amount::Specified(a) => a,
        }
    }
}

impl<S: ErrorSpan> DecodeScalar<S> for Amount {
    fn type_check(type_name: &Option<knus::span::Spanned<knus::ast::TypeName, S>>, ctx: &mut knus::decode::Context<S>) {
        if let Some(typ) = type_name {
            ctx.emit_error(DecodeError::TypeName {
                span: typ.span().clone(),
                found: Some(typ.deref().clone()),
                expected: ExpectedType::no_type(),
                rust_type: "String",
            });
        }
    }

    fn raw_decode(
        value: &knus::span::Spanned<knus::ast::Literal, S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, knus::errors::DecodeError<S>> {
        match &**value {
            Literal::String(s) if &**s == "auto" => Ok(Self::Auto),
            Literal::Int(_) => {
                let n = i32::raw_decode(value, ctx)?;
                Ok(Amount::Specified(n))
            }
            _ => {
                ctx.emit_error(DecodeError::unsupported(value, "expected `auto` or an integer"));
                Ok(Amount::Auto)
            }
        }    
    }
}
