use serde::{Deserialize, de};
use serde_untagged::UntaggedEnumVisitor;

use crate::amount::Amount::Auto;


#[derive(Debug, Clone, Copy)]
pub enum Amount {
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
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        UntaggedEnumVisitor::new()
            .u32(|x| Ok(Amount::Specified(x as i32)))
            .string(|string| match string {
                "auto" => Ok(Amount::Auto),
                _ => Err(de::Error::invalid_value(
                    de::Unexpected::Str(string),
                    &r#"integer or "auto""#,
                )),
            })
            .deserialize(deserializer)
    }
}
