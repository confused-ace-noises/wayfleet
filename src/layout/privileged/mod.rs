use std::sync::{Arc, RwLock};

use smithay::{utils::{Logical, Point, Rectangle, Size}};
use wayfleet_config::{amount::Amount, padding::Padding, size::Spaces};

use crate::{animations::AnimationHandle, layout::{CropRect, privileged::tile::Tile}, state::OutputState};

pub mod tile;
pub mod insert;
pub mod utils;
pub mod recalc;
pub mod moving;
pub mod focus;
pub mod resize;

#[derive(Debug)]
pub struct Privileged {
    pub viewport: CropRect,
    pub right_shift: i32,
    pub privileged: Vec<Vec<Tile>>,
    pub animation: AnimationHandle,
    pub map_offset: i32,
    pub focused: Option<(usize, usize)>,
    pub spaces: Spaces,
    pub std_size: Size<i32, Logical>,
}

pub enum Height {
    New(Amount),
    FromOld((OutputState, Rectangle<i32, Logical>)),
}

impl Privileged {
    pub fn output_state_to_rect(output: &OutputState, padding: &Padding, default_height: Height) -> Rectangle<i32, Logical> {
        let output = output.logical_size();

        let height = match default_height {
            // default heigth: 40%
            // default width: 100%
            Height::New(amount) => amount.unwrap_or_else(|| output.h * 40 / 100) - padding.top - padding.down,
            Height::FromOld((_, rectangle)) => rectangle.size.h,
        };

        let point: Point<i32, Logical> = match default_height {
            Height::New(_) => Point::new(padding.left, padding.top),
            Height::FromOld((_, rectangle)) => rectangle.loc,
        };

        let width = match default_height {
            Height::New(_) => output.w - padding.left - padding.right,
            Height::FromOld((old_state, rect)) => {
                let old_logical = old_state.logical_size().w;
                let new_width = output.w;

                let delta = new_width - old_logical;

                rect.size.w + delta
            },
        };

        Rectangle { loc: point, size: Size::new(width, height) }
    }

    pub fn new(
        wayfleet_config::Privileged {
            height,
            spaces,
            padding,
            standard_width,
        }: &wayfleet_config::Privileged,
        output: &OutputState,
        animation: AnimationHandle,
    ) -> Self {

        let viewport = Self::output_state_to_rect(output, padding, Height::New(*height));

        let spaces = spaces.unwrap_or_else(|| Spaces { horizontal: 0, vertical: 0 });

        Self {
            privileged: vec![],
            right_shift: 0,
            viewport: Arc::new(RwLock::new(viewport)),
            animation,
            map_offset: viewport.size.h + padding.top + padding.down, // note: redoing this from before but tbh not adding vertical padding here makes no sense
            focused: None,
            spaces,
            std_size: Size::new(standard_width.unwrap_or(viewport.size.w * 60 / 100), viewport.size.h)
        }
    }
}