use smithay::{utils::{Logical, Point, Rectangle, Size}};
use wayfleet_config::{padding::Padding, size::Spaces};

use crate::{animations::AnimationHandle, layout::privileged::tile::Tile, state::OutputState};

pub mod tile;
pub mod insert;
pub mod utils;
pub mod recalc;
pub mod moving;
pub mod focus;

#[derive(Debug)]
pub struct Privileged {
    pub viewport: Rectangle<i32, Logical>,
    pub right_shift: i32,
    pub privileged: Vec<Vec<Tile>>,
    pub animation: AnimationHandle,
    pub map_offset: i32,
    pub focused: Option<(usize, usize)>,
    pub spaces: Spaces,
    pub std_size: Size<i32, Logical>,
}

impl Privileged {
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

        let output = output.logical_size();

        // default heigth: 40%
        // default width: 100%
        let mut height = height.unwrap_or_else(|| output.h * 40 / 100);

        let map_offset = height; // dont make a single pixel overlap

        let Padding { left, right, top, down } = padding;
        
        let point = Point::<_, Logical>::new(*left, *top);

        let mut width = output.w;

        height -= top + down;
        width -= left + right;

        let viewport = Rectangle::new(point, Size::new(width, height));

        let spaces = spaces.unwrap_or_else(|| Spaces { horizontal: 0, vertical: 0 });

        Self {
            privileged: vec![],
            right_shift: 0,
            viewport,
            animation,
            map_offset,
            focused: None,
            spaces,
            std_size: Size::new(standard_width.unwrap_or(width * 60 / 100), height)
        }
    }
}