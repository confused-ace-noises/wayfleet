use knus::Decode;
use miette::{NamedSource, SourceSpan};

use crate::{Spanned, amount::Amount, error::{ConfigError, ValidationError}, padding::Padding, size::{Grid, GridRecv, Size, SizeRepr, SizeReprRecv, Spaces}};

pub struct Map {
    pub size: Size<Grid>,
    pub cells: Size<SizeRepr>,
    pub spaces: Size<Spaces>,
    pub margins: Padding,
}

#[derive(Debug, Decode)]
pub(crate) struct MapSpanned {
    #[knus(child)]
    pub size: Spanned<Size<GridRecv>>,
    
    #[knus(child)]
    pub cells: Spanned<Size<SizeReprRecv>>,
    
    #[knus(child, default)]
    pub spaces: Size<Spaces>,
    
    #[knus(child, default)]
    pub margins: Padding,
}

impl MapSpanned {
    pub fn into_map(self, src: NamedSource<String>) -> Result<Map, ConfigError> {
        let Self { size, cells, spaces, margins } = self;

        let mut loc1a = None::<SourceSpan>;
        let mut loc2a = None::<SourceSpan>;

        let mut loc1h = None::<SourceSpan>;
        let mut loc2h = None::<SourceSpan>;

        let mut loc1w = None::<SourceSpan>;
        let mut loc2w = None::<SourceSpan>;
        
        match size.as_ref() {
            Size::Specified(GridRecv { rows, columns }) => {
                if matches!(rows.as_ref(), Amount::Auto) {
                    loc1h = Some(*rows.span())
                }

                if matches!(columns.as_ref(), Amount::Auto) {
                    loc1w = Some(*columns.span())
                }
            },
            Size::Auto => loc1a = Some(*size.span()),
        }

        match cells.as_ref() {
            Size::Specified(SizeReprRecv { width, height }) => {
                if matches!(height.as_ref(), Amount::Auto) {
                    loc2h = Some(*height.span())
                }

                if matches!(width.as_ref(), Amount::Auto) {
                    loc2w = Some(*width.span())
                }
            },
            Size::Auto => loc2a = Some(*cells.span()),
        }

        if let Some(loc1) = loc1a && let Some(loc2) = loc2a {
            Err(ValidationError::MapValidationAll { src, loc1, loc2 })?

        } else if let Some(loc1) = loc1h && let Some(loc2) = loc2h {
            Err(ValidationError::MapValidationRows { src, loc1, loc2 })?
        
        } else if let Some(loc1) = loc1w && let Some(loc2) = loc2w {
            Err(ValidationError::MapValidationColumns { src, loc1, loc2 })?
        
        } else {
            if loc2a.is_some() {
                loc1a = loc2a
            }
    
            if loc2h.is_some() {
                loc1h = loc2h
            }
    
            if loc2w.is_some() {
                loc1w = loc2w
            }
    
            if let Some(loc1) = loc1a && let Some(loc2) = loc1h {
                Err(ValidationError::MapValidationGeneric { src, loc1, loc2 })?
    
            } else if let Some(loc1) = loc1a && let Some(loc2) = loc1w {
                Err(ValidationError::MapValidationGeneric { src, loc1, loc2 })?
            }
        } 

        Ok(Map {
            size: size.value.into(),
            cells: cells.value.into(),
            spaces: spaces.into(),
            margins,
        })
    }
}


// impl<S: ErrorSpan> Decode<S> for MapSpanned {
//     fn decode_node(node: &knus::ast::SpannedNode<S>, ctx: &mut knus::decode::Context<S>) -> Result<Self, knus::errors::DecodeError<S>> {
//         if node.arguments.is_empty() {
//             ctx.emit_error(DecodeError::unexpected(node, "argument", "map doesn't accept arguments"));
//         }

//         let mut size = None;
//         let mut cells = None;
//         let mut spaces = None;
//         let mut padding = None;

//         for child in node.children() {
//             // match_child!(child, ctx; size: Size::<Grid>, cells: Size::<SizeRepr>);
//             match child.node_name.as_ref() {
//                 "size" => {
//                     if size.is_some() {
//                         ctx.emit_error(DecodeError::unexpected(child, "node", "can't specify `size` twice"));
//                     }
//                     size = Some(SizeSpanned::<Grid>::decode_node(child, ctx)?)
//                 },


//                 "cells" => {
//                     if cells.is_some() {
//                         ctx.emit_error(DecodeError::unexpected(child, "node", "can't specify `cells` twice"));
//                     }
//                     cells = Some(SizeSpanned::<SizeRepr>::decode_node(child, ctx)?)
//                 },


//                 "spaces" => {
//                     if spaces.is_some() {
//                         ctx.emit_error(DecodeError::unexpected(child, "node", "can't specify `spaces` twice"));
//                     }
//                     spaces = Some(SizeSpanned::<Spaces>::decode_node(child, ctx)?)
//                 },

//                 "margins" => {
//                     if padding.is_some() {
//                         ctx.emit_error(DecodeError::unexpected(child, "node", "can't specify `margins` twice"));
//                     }
//                     padding = Some(Padding::decode_node(child, ctx)?)
//                 }

//                 name => ctx.emit_error(DecodeError::unexpected(child, "node", format!("unexpected node `{name}`"))),
//             }
//         }

//         let mut size = size.ok_or_else(|| DecodeError::missing(node, "missing `size`"))?;
//         let mut cells = cells.ok_or_else(|| DecodeError::missing(node, "missing `cells`"))?;
//         let mut spaces = spaces.unwrap_or_default();
//         let mut padding = padding.unwrap_or_default();


        



//         // if let Some(children) = &node.children {
//         //     let children = children.deref();


//         // }

//         todo!()
//     }
// } 



// impl MapRecv {
//     pub fn into_map(self, src: NamedSource<String>) -> Result<Map, ConfigError> {
//         let MapRecv { size, cells, spaces, margins } = self;

//         let mut loc1a = None::<SourceSpan>;
//         let mut loc2a = None::<SourceSpan>;

//         let mut loc1h = None::<SourceSpan>;
//         let mut loc2h = None::<SourceSpan>;

//         let mut loc1w = None::<SourceSpan>;
//         let mut loc2w = None::<SourceSpan>;
        
//         match size.as_ref() {
//             Size::Specified(GridRecv { rows, columns }) => {
//                 if matches!(rows.as_ref(), Amount::Auto) {
//                     loc1h = Some(rows.span().into())
//                 }

//                 if matches!(columns.as_ref(), Amount::Auto) {
//                     loc1w = Some(columns.span().into())
//                 }
//             },
//             Size::Auto => loc1a = Some(size.span().into()),
//         }

//         match cells.as_ref() {
//             Size::Specified(SizeReprRecv { width, height }) => {
//                 if matches!(height.as_ref(), Amount::Auto) {
//                     loc2h = Some(height.span().into())
//                 }

//                 if matches!(width.as_ref(), Amount::Auto) {
//                     loc2w = Some(width.span().into())
//                 }
//             },
//             Size::Auto => loc2a = Some(cells.span().into()),
//         }

//         if let Some(loc1) = loc1a && let Some(loc2) = loc2a {
//             Err(ValidationError::MapValidationAll { src, loc1, loc2 })?

//         } else if let Some(loc1) = loc1h && let Some(loc2) = loc2h {
//             Err(ValidationError::MapValidationRows { src, loc1, loc2 })?
        
//         } else if let Some(loc1) = loc1w && let Some(loc2) = loc2w {
//             Err(ValidationError::MapValidationColumns { src, loc1, loc2 })?
        
//         } else {
//             if loc2a.is_some() {
//                 loc1a = loc2a
//             }
    
//             if loc2h.is_some() {
//                 loc1h = loc2h
//             }
    
//             if loc2w.is_some() {
//                 loc1w = loc2w
//             }
    
//             if let Some(loc1) = loc1a && let Some(loc2) = loc1h {
//                 Err(ValidationError::MapValidationGeneric { src, loc1, loc2 })?
    
//             } else if let Some(loc1) = loc1a && let Some(loc2) = loc1w {
//                 Err(ValidationError::MapValidationGeneric { src, loc1, loc2 })?
//             }
//         } 

//         Ok(Map {
//             size: size.into_inner().into(),
//             cells: cells.into_inner().into(),
//             spaces: spaces.into(),
//             margins,
//         })
//     }
// }