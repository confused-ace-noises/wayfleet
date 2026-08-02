use std::ops::Deref;

use knus::{DecodeScalar, ast::Literal, decode::Kind, errors::{DecodeError, ExpectedType}, traits::ErrorSpan};

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
                ctx.emit_error(DecodeError::unsupported(value, "expected `\"auto\"` or an integer"));
                Ok(Amount::Auto)
            }
        }    
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SetSizeAmount {
    Delta{
        amount: i32,
        resize: ResizeSizeType,
    },
    Final{ 
        amount: i32,
        resize: ResizeSizeType,
    },
}

impl SetSizeAmount {
    pub fn get_final_resize(&self, inital: i32, screen_size: i32) -> i32 {
        match *self {
            SetSizeAmount::Final { amount, resize } => {
                resize.to_pixels(amount, screen_size)
            },
            SetSizeAmount::Delta { amount, resize } => {
                let delta = resize.to_pixels(amount, screen_size);

                inital + delta
            },
        }
    }

    pub fn get_delta_resize(&self, inital: i32, screen_size: i32) -> i32 {
        match *self {
            SetSizeAmount::Final { amount, resize } => {
                let dest = resize.to_pixels(amount, screen_size);
                dest - inital
            },
            SetSizeAmount::Delta { amount, resize } => {
                resize.to_pixels(amount, screen_size)
            },
        }
    }
}

impl<S: ErrorSpan> DecodeScalar<S> for SetSizeAmount {
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
        value: &knus::span::Spanned<Literal, S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        if let Literal::String(string) = &**value {

            let resize: ResizeSizeType;
            let num_str: &str;

            if let Some(rest) = string.strip_suffix("px") {
                num_str = rest;
                resize = ResizeSizeType::Px;
            } else if let Some(rest) = string.strip_suffix('%') {
                num_str = rest;
                resize = ResizeSizeType::ScreenPercent;
            } else {
                ctx.emit_error(DecodeError::unsupported(value, "expected either `%` or `px`"));
                num_str = "+0"; // +0px is a no-op
                resize = ResizeSizeType::Px;
            };

            let mut is_delta = num_str.strip_prefix("+").is_some() || num_str.strip_prefix("-").is_some();


            if num_str.strip_prefix("+").is_some() || num_str.strip_prefix("-").is_some() {
                is_delta = true;
            }

            let num = num_str.parse::<i32>();

            let num = match num {
                Ok(n) => n,
                Err(_) => {
                    ctx.emit_error(DecodeError::unsupported(value, format!("expected a number, found {}", num_str)));
                    0
                },
            };

            if is_delta {
                Ok(Self::Delta { amount: num, resize })
            } else {
                Ok(Self::Final { amount: num, resize })
            }
        } else {
            ctx.emit_error(DecodeError::scalar_kind(Kind::String, value));
            Ok(Self::Delta { amount: 0, resize: ResizeSizeType::Px })
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResizeSizeType {
    ScreenPercent,
    Px, 
}

impl ResizeSizeType {
    pub fn to_pixels(&self, amount: i32, screen_size: i32) -> i32 {
        match self {
            ResizeSizeType::Px => amount,
            ResizeSizeType::ScreenPercent => ((amount as f64/100.)*screen_size as f64).round_ties_even() as i32,
        }
    }
}