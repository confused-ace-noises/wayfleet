use serde::{Deserialize, de};
use serde_untagged::UntaggedEnumVisitor;
use toml::Spanned;

use crate::amount::Amount;

#[derive(Debug, Deserialize)]
pub struct Grid {
    pub rows: Amount,
    pub columns: Amount,
}

#[derive(Debug, Deserialize)]
pub struct SizeRepr {
    pub height: Amount, 
    pub width: Amount
}

#[derive(Debug, Deserialize, Default, Copy, Clone)]
pub struct Spaces {
    pub horizontal: u32,
    pub vertical: u32,
}

impl From<SpacesRecv> for Spaces {
    fn from(SpacesRecv { horizontal, vertical }: SpacesRecv) -> Self {
        Self {
            horizontal: horizontal.into_inner().unwrap_or(0),
            vertical: vertical.into_inner().unwrap_or(0)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GridRecv {
    pub rows: Spanned<Amount>,
    pub columns: Spanned<Amount>,
}

impl From<GridRecv> for Grid {
    fn from(GridRecv { rows, columns }: GridRecv) -> Self {
        Grid { rows: rows.into_inner(), columns: columns.into_inner() }
    }
}

#[derive(Debug, Deserialize)]
pub struct SizeReprRecv {
    pub height: Spanned<Amount>, 
    pub width: Spanned<Amount>
}

impl From<SizeReprRecv> for SizeRepr {
    fn from(SizeReprRecv { height, width }: SizeReprRecv) -> Self {
        SizeRepr { height: height.into_inner(), width: width.into_inner() }
    }
}

#[derive(Debug, Deserialize)]
pub struct SpacesRecv {
    pub horizontal: Spanned<Option<u32>>,
    pub vertical: Spanned<Option<u32>>,
}

#[derive(Debug, Default)]
pub enum Size<T> {
    Specified(T),

    #[default]
    Auto,
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

impl<'de, T> Deserialize<'de> for Size<T> 
where 
    T: Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        UntaggedEnumVisitor::new()
            .map(|map| {
                map.deserialize::<T>().map(Size::Specified)
            })
            .string(|string| match string {
                "auto" => Ok(Size::Auto),
                _ => Err(de::Error::invalid_value(
                    de::Unexpected::Str(string),
                    &r#"integer or "auto""#,
                )),
            })
            .deserialize(deserializer)
    }
}

