//! Flexbox style definitions.

use serde::{Deserialize, Serialize};

/// Flexbox style for layout nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexStyle {
    // Display and positioning
    pub display: Display,
    pub position: Position,
    pub overflow: Overflow,

    // Flex container properties
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_content: AlignContent,

    // Flex item properties
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_self: AlignSelf,

    // Dimensions
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,

    // Spacing
    pub margin: Edges,
    pub padding: Edges,
    pub gap: Gap,

    // Border (affects layout but not rendered here)
    pub border: Edges,

    // Position offsets (for absolute positioning)
    pub inset: Inset,
}

impl Default for FlexStyle {
    fn default() -> Self {
        Self {
            display: Display::default(),
            position: Position::default(),
            overflow: Overflow::default(),
            flex_direction: FlexDirection::default(),
            flex_wrap: FlexWrap::default(),
            justify_content: JustifyContent::default(),
            align_items: AlignItems::default(),
            align_content: AlignContent::default(),
            flex_grow: 0.0,
            flex_shrink: 1.0, // CSS default is 1, not 0
            flex_basis: Dimension::default(),
            align_self: AlignSelf::default(),
            width: Dimension::default(),
            height: Dimension::default(),
            min_width: Dimension::default(),
            min_height: Dimension::default(),
            max_width: Dimension::default(),
            max_height: Dimension::default(),
            margin: Edges::default(),
            padding: Edges::default(),
            gap: Gap::default(),
            border: Edges::default(),
            inset: Inset::default(),
        }
    }
}

impl FlexStyle {
    /// Create a new flex style with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert to taffy Style.
    pub fn to_taffy(&self) -> taffy::Style {
        taffy::Style {
            display: self.display.to_taffy(),
            position: self.position.to_taffy(),
            overflow: taffy::Point {
                x: self.overflow.to_taffy(),
                y: self.overflow.to_taffy(),
            },
            flex_direction: self.flex_direction.to_taffy(),
            flex_wrap: self.flex_wrap.to_taffy(),
            justify_items: Some(taffy::JustifyItems::Start),
            justify_self: Some(taffy::JustifySelf::Start),
            justify_content: Some(self.justify_content.to_taffy()),
            align_items: Some(self.align_items.to_taffy()),
            align_self: self.align_self.to_taffy(),
            align_content: Some(self.align_content.to_taffy()),
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            flex_basis: self.flex_basis.to_taffy(),
            size: taffy::Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            min_size: taffy::Size {
                width: self.min_width.to_taffy(),
                height: self.min_height.to_taffy(),
            },
            max_size: taffy::Size {
                width: self.max_width.to_taffy(),
                height: self.max_height.to_taffy(),
            },
            aspect_ratio: None,
            margin: self.margin.to_taffy(),
            padding: self.padding.to_taffy_no_auto(),
            gap: self.gap.to_taffy(),
            border: self.border.to_taffy_no_auto(),
            inset: self.inset.to_taffy(),
            scrollbar_width: 0.0,
            ..Default::default()
        }
    }
}

/// Display type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Display {
    #[default]
    Flex,
    None,
}

impl Display {
    fn to_taffy(self) -> taffy::Display {
        match self {
            Display::Flex => taffy::Display::Flex,
            Display::None => taffy::Display::None,
        }
    }
}

/// Position type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

impl Position {
    fn to_taffy(self) -> taffy::Position {
        match self {
            Position::Relative => taffy::Position::Relative,
            Position::Absolute => taffy::Position::Absolute,
        }
    }
}

/// Overflow behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

impl Overflow {
    fn to_taffy(self) -> taffy::Overflow {
        match self {
            Overflow::Visible => taffy::Overflow::Visible,
            Overflow::Hidden => taffy::Overflow::Hidden,
            Overflow::Scroll => taffy::Overflow::Scroll,
        }
    }
}

/// Flex direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl FlexDirection {
    fn to_taffy(self) -> taffy::FlexDirection {
        match self {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        }
    }
}

/// Flex wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

impl FlexWrap {
    fn to_taffy(self) -> taffy::FlexWrap {
        match self {
            FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::FlexWrap::Wrap,
            FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
        }
    }
}

/// Justify content (main axis alignment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl JustifyContent {
    fn to_taffy(self) -> taffy::JustifyContent {
        match self {
            JustifyContent::FlexStart => taffy::JustifyContent::FlexStart,
            JustifyContent::FlexEnd => taffy::JustifyContent::FlexEnd,
            JustifyContent::Center => taffy::JustifyContent::Center,
            JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
            JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
            JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
        }
    }
}

/// Align items (cross axis alignment for children).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlignItems {
    Stretch,
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl AlignItems {
    fn to_taffy(self) -> taffy::AlignItems {
        match self {
            AlignItems::Stretch => taffy::AlignItems::Stretch,
            AlignItems::FlexStart => taffy::AlignItems::FlexStart,
            AlignItems::FlexEnd => taffy::AlignItems::FlexEnd,
            AlignItems::Center => taffy::AlignItems::Center,
            AlignItems::Baseline => taffy::AlignItems::Baseline,
        }
    }
}

/// Align content (cross axis alignment for wrapped lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlignContent {
    Stretch,
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
}

impl AlignContent {
    fn to_taffy(self) -> taffy::AlignContent {
        match self {
            AlignContent::Stretch => taffy::AlignContent::Stretch,
            AlignContent::FlexStart => taffy::AlignContent::FlexStart,
            AlignContent::FlexEnd => taffy::AlignContent::FlexEnd,
            AlignContent::Center => taffy::AlignContent::Center,
            AlignContent::SpaceBetween => taffy::AlignContent::SpaceBetween,
            AlignContent::SpaceAround => taffy::AlignContent::SpaceAround,
        }
    }
}

/// Align self (cross axis alignment for single item).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlignSelf {
    #[default]
    Auto,
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl AlignSelf {
    fn to_taffy(self) -> Option<taffy::AlignSelf> {
        match self {
            AlignSelf::Auto => None,
            AlignSelf::Stretch => Some(taffy::AlignSelf::Stretch),
            AlignSelf::FlexStart => Some(taffy::AlignSelf::FlexStart),
            AlignSelf::FlexEnd => Some(taffy::AlignSelf::FlexEnd),
            AlignSelf::Center => Some(taffy::AlignSelf::Center),
            AlignSelf::Baseline => Some(taffy::AlignSelf::Baseline),
        }
    }
}

/// A dimension value.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum Dimension {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

impl Dimension {
    /// Create a points dimension.
    pub fn points(value: f32) -> Self {
        Dimension::Points(value)
    }

    /// Create a percent dimension.
    pub fn percent(value: f32) -> Self {
        Dimension::Percent(value)
    }

    fn to_taffy(self) -> taffy::Dimension {
        match self {
            Dimension::Auto => taffy::Dimension::Auto,
            Dimension::Points(v) => taffy::Dimension::Length(v),
            Dimension::Percent(v) => taffy::Dimension::Percent(v / 100.0),
        }
    }
}

/// Edge values for margin, padding, border.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Edges {
    pub top: LengthPercentageAuto,
    pub right: LengthPercentageAuto,
    pub bottom: LengthPercentageAuto,
    pub left: LengthPercentageAuto,
}

impl Edges {
    /// Create edges with all sides the same.
    pub fn all(value: f32) -> Self {
        Self {
            top: LengthPercentageAuto::Points(value),
            right: LengthPercentageAuto::Points(value),
            bottom: LengthPercentageAuto::Points(value),
            left: LengthPercentageAuto::Points(value),
        }
    }

    /// Create edges with vertical and horizontal values.
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: LengthPercentageAuto::Points(vertical),
            right: LengthPercentageAuto::Points(horizontal),
            bottom: LengthPercentageAuto::Points(vertical),
            left: LengthPercentageAuto::Points(horizontal),
        }
    }

    fn to_taffy(self) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            top: self.top.to_taffy(),
            right: self.right.to_taffy(),
            bottom: self.bottom.to_taffy(),
            left: self.left.to_taffy(),
        }
    }

    /// Convert to taffy Rect<LengthPercentage> for padding/border.
    fn to_taffy_no_auto(self) -> taffy::Rect<taffy::LengthPercentage> {
        taffy::Rect {
            top: self.top.to_taffy_no_auto(),
            right: self.right.to_taffy_no_auto(),
            bottom: self.bottom.to_taffy_no_auto(),
            left: self.left.to_taffy_no_auto(),
        }
    }
}

/// Length, percentage, or auto value.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum LengthPercentageAuto {
    #[default]
    Auto,
    Points(f32),
    Percent(f32),
}

impl LengthPercentageAuto {
    fn to_taffy(self) -> taffy::LengthPercentageAuto {
        match self {
            LengthPercentageAuto::Auto => taffy::LengthPercentageAuto::Auto,
            LengthPercentageAuto::Points(v) => taffy::LengthPercentageAuto::Length(v),
            LengthPercentageAuto::Percent(v) => taffy::LengthPercentageAuto::Percent(v / 100.0),
        }
    }

    /// Convert to LengthPercentage, treating Auto as zero.
    fn to_taffy_no_auto(self) -> taffy::LengthPercentage {
        match self {
            LengthPercentageAuto::Auto => taffy::LengthPercentage::Length(0.0),
            LengthPercentageAuto::Points(v) => taffy::LengthPercentage::Length(v),
            LengthPercentageAuto::Percent(v) => taffy::LengthPercentage::Percent(v / 100.0),
        }
    }
}

/// Gap values for row and column gaps.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Gap {
    pub row: f32,
    pub column: f32,
}

impl Gap {
    /// Create gap with same value for rows and columns.
    pub fn all(value: f32) -> Self {
        Self {
            row: value,
            column: value,
        }
    }

    fn to_taffy(self) -> taffy::Size<taffy::LengthPercentage> {
        taffy::Size {
            width: taffy::LengthPercentage::Length(self.column),
            height: taffy::LengthPercentage::Length(self.row),
        }
    }
}

/// Inset values for absolute positioning.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Inset {
    pub top: LengthPercentageAuto,
    pub right: LengthPercentageAuto,
    pub bottom: LengthPercentageAuto,
    pub left: LengthPercentageAuto,
}

impl Inset {
    fn to_taffy(self) -> taffy::Rect<taffy::LengthPercentageAuto> {
        taffy::Rect {
            top: self.top.to_taffy(),
            right: self.right.to_taffy(),
            bottom: self.bottom.to_taffy(),
            left: self.left.to_taffy(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_style_default() {
        let style = FlexStyle::new();
        assert_eq!(style.flex_direction, FlexDirection::Row);
        assert_eq!(style.display, Display::Flex);
    }

    #[test]
    fn test_flex_style_to_taffy() {
        let mut style = FlexStyle::new();
        style.flex_direction = FlexDirection::Column;
        style.width = Dimension::Points(100.0);

        let taffy_style = style.to_taffy();
        assert_eq!(taffy_style.flex_direction, taffy::FlexDirection::Column);
    }

    #[test]
    fn test_edges_all() {
        let edges = Edges::all(10.0);
        assert_eq!(edges.top, LengthPercentageAuto::Points(10.0));
        assert_eq!(edges.right, LengthPercentageAuto::Points(10.0));
    }
}
