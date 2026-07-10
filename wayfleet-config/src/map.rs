use miette::{NamedSource, SourceSpan};
use serde::Deserialize;
use toml::Spanned;

use crate::{amount::Amount, error::{ConfigError, ValidationError}, padding::Margins, size::{Grid, GridRecv, Size, SizeRepr, SizeReprRecv, Spaces, SpacesRecv}};


#[derive(Debug, Deserialize)]
pub struct MapRecv {
    pub size: Spanned<Size<GridRecv>>,
    pub cells: Spanned<Size<SizeReprRecv>>,
    pub spaces: Size<SpacesRecv>,

    #[serde(default)]
    pub margins: Margins,
}

#[derive(Debug)]
pub struct Map {
    pub size: Size<Grid>,
    pub cells: Size<SizeRepr>,
    pub spaces: Size<Spaces>,
    pub margins: Margins,
}

impl MapRecv {
    pub fn into_map(self, src: NamedSource<String>) -> Result<Map, ConfigError> {
        let MapRecv { size, cells, spaces, margins } = self;

        let mut loc1a = None::<SourceSpan>;
        let mut loc2a = None::<SourceSpan>;

        let mut loc1h = None::<SourceSpan>;
        let mut loc2h = None::<SourceSpan>;

        let mut loc1w = None::<SourceSpan>;
        let mut loc2w = None::<SourceSpan>;
        
        match size.as_ref() {
            Size::Specified(GridRecv { rows, columns }) => {
                if matches!(rows.as_ref(), Amount::Auto) {
                    loc1h = Some(rows.span().into())
                }

                if matches!(columns.as_ref(), Amount::Auto) {
                    loc1w = Some(columns.span().into())
                }
            },
            Size::Auto => loc1a = Some(size.span().into()),
        }

        match cells.as_ref() {
            Size::Specified(SizeReprRecv { width, height }) => {
                if matches!(height.as_ref(), Amount::Auto) {
                    loc2h = Some(height.span().into())
                }

                if matches!(width.as_ref(), Amount::Auto) {
                    loc2w = Some(width.span().into())
                }
            },
            Size::Auto => loc2a = Some(cells.span().into()),
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
            size: size.into_inner().into(),
            cells: cells.into_inner().into(),
            spaces: spaces.into(),
            margins,
        })
    }
}