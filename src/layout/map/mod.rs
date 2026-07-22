pub mod tile;
pub mod coordinate;
pub mod insert;
pub mod utils;
pub mod moving;
pub mod swap;
pub mod resize;
pub mod focus;

use smithay::utils::{Logical, Point};
use tile::Tile;
use coordinate::Coordinate;
use wayfleet_config::{amount::Amount, size::Spaces};

use crate::{animations::AnimationHandle, state::OutputState};

#[derive(Debug)]
pub struct Map {
    pub map: Vec<Vec<Option<Tile>>>,
    pub first_available: Option<Coordinate>,
    pub rows: usize,
    pub columns: usize,
    pub cell_height: i32,
    pub cell_width: i32,
    pub spaces: Spaces,
    pub offset: Point<i32, Logical>,
    pub animation: AnimationHandle,
    pub focus: Option<Coordinate>,
}

impl Map {
    pub fn new(
        config: &wayfleet_config::Map,
        animation: AnimationHandle,
        OutputState { size: output_size, scale_factor, ..}: &OutputState,
        privileged_offset: i32
    ) -> Self {
        let wayfleet_config::Map { size, cells, spaces, margins } = config;

        let mut output_size = output_size.to_logical(*scale_factor);

        output_size.h -= privileged_offset;

        let spaces = (*spaces).unwrap_or_else(|| Spaces { horizontal: 0, vertical: 0} );

        let (rows, columns) = match size {
            wayfleet_config::size::Size::Specified(wayfleet_config::size::Grid { rows, columns }) => {
                let first = if let Amount::Specified(rows) = rows {
                    *rows
                } else {
                    (output_size.h + spaces.vertical as i32) / (cells.unwrap_ref().height.unwrap() + spaces.vertical.max(1) as i32)
                };

                let second = if let Amount::Specified(cols) = columns {
                    *cols
                } else {
                    ((output_size.w + spaces.horizontal as i32) as f64 / (cells.unwrap_ref().width.unwrap() + spaces.horizontal.max(1) as i32)as f64) as i32
                };
                
                (
                    first,
                    second
                )
            },
            wayfleet_config::size::Size::Auto => {
                (
                    (output_size.h + spaces.vertical as i32) / (cells.unwrap_ref().height.unwrap() + spaces.vertical.max(1) as i32),
                    ((output_size.w + spaces.horizontal as i32) as f64 / (cells.unwrap_ref().width.unwrap() + spaces.horizontal.max(1) as i32)as f64) as i32
                )
            },
        };

        let (cell_height, cell_width) = match cells {
            wayfleet_config::size::Size::Specified(wayfleet_config::size::SizeRepr { height, width }) => {
                let first = if let Amount::Specified(heigth) = height {
                    *heigth
                } else {
                    (output_size.h + spaces.vertical as i32) / rows - spaces.vertical.max(1) as i32
                };

                let second = if let Amount::Specified(width) = width {
                    *width
                } else {
                    (output_size.w + spaces.horizontal as i32) / columns - spaces.horizontal.max(1) as i32
                };
                
                (
                    first,
                    second
                )
            },
            wayfleet_config::size::Size::Auto => {
                (
                    (output_size.h + spaces.vertical as i32) / rows - spaces.vertical.max(1) as i32,
                    (output_size.w + spaces.horizontal as i32) / columns - spaces.horizontal.max(1) as i32
                )
            },
        };

        assert_ne!(rows, 0);
        assert_ne!(columns, 0);
        assert!(cell_height > 0);
        assert!(cell_width > 0);

        let columns = columns as usize;
        let rows = rows as usize;

        // TODO: proper margins

        dbg!(Self {
            map: vec![vec![None; columns]; rows],
            first_available: Some([0, 0].into()),
            rows,
            columns,
            cell_height,
            cell_width,
            spaces,
            offset: Point::new(0, privileged_offset),
            animation,
            focus: None
        })
    }
}