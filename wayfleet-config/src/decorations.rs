use std::ops::Deref;

use knus::{Decode, DecodeScalar, ast::Literal, errors::{DecodeError, ExpectedType}, traits::ErrorSpan};
use smithay::backend::renderer::Color32F;

#[derive(Debug, Decode)]
pub struct Decorations {
    #[knus(child)]
    pub border: Border,
}

impl Decorations {
    pub fn dummy() -> Self {
        Self { border: Border { active_color: Color::BLACK, inactive_color: Color::BLACK, width: 0, corner_radius: 0 } }
    }
}

#[derive(Debug, Decode)]
pub struct Border {
    #[knus(child, unwrap(argument))]
    pub active_color: Color,

    #[knus(child, unwrap(argument))]
    pub inactive_color: Color,

    #[knus(child, unwrap(argument))]
    pub width: u32,
    
    #[knus(child, unwrap(argument), default)]
    pub corner_radius: u32,
}

#[derive(Debug, Clone)]
pub struct Color(pub [f32; 4]);

impl From<Color> for Color32F {
    fn from(value: Color) -> Self {
        Color32F::from(value.0)
    }
}

impl Color {
    pub const BLACK: Self = Color([0., 0., 0., 1.]);
}

impl<S: ErrorSpan> DecodeScalar<S> for Color {
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
        fn exit_with_black<S: ErrorSpan>(
            value: &knus::span::Spanned<knus::ast::Literal, S>,
            ctx: &mut knus::decode::Context<S>,
        ) -> Result<Color, knus::errors::DecodeError<S>> {
            ctx.emit_error(DecodeError::unexpected(value, "hex color", "Expected a hex color in the form of #rrggbb or #rrggbbaa"));
            Ok(Color([0.,0.,0.,1.]))
        }

        let Literal::String(string) = &**value else { return exit_with_black(value, ctx) };

        let Some(x) = string.strip_prefix('#') else { return exit_with_black(value, ctx) };

        
        let Some(r) = x.get(0..2).and_then(|x| u8::from_str_radix(x, 16).ok()) else { return exit_with_black(value, ctx) };
        let Some(g) = x.get(2..4).and_then(|x| u8::from_str_radix(x, 16).ok()) else { return exit_with_black(value, ctx) };
        let Some(b) = x.get(4..6).and_then(|x| u8::from_str_radix(x, 16).ok()) else { return exit_with_black(value, ctx) };
        let a       = x.get(6..8).and_then(|x| u8::from_str_radix(x, 16).ok()).unwrap_or(255);

        Ok(Self([r as f32 / 255., g as f32 / 255., b as f32 / 255., a as f32 / 255.]))
    }
}