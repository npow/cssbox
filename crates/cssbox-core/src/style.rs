//! Computed CSS style types for layout.

use crate::values::{LengthPercentage, LengthPercentageAuto, LengthPercentageNone};

/// CSS `display` outer type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayOuter {
    Block,
    Inline,
    None,
}

/// CSS `display` inner type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayInner {
    Flow,
    FlowRoot,
    Flex,
    Grid,
    Table,
    TableRowGroup,
    TableRow,
    TableCell,
    TableColumn,
    TableColumnGroup,
    TableCaption,
    TableHeaderGroup,
    TableFooterGroup,
}

/// Resolved `display` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    pub outer: DisplayOuter,
    pub inner: DisplayInner,
}

impl Display {
    pub const BLOCK: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::Flow,
    };
    pub const INLINE: Self = Self {
        outer: DisplayOuter::Inline,
        inner: DisplayInner::Flow,
    };
    pub const INLINE_BLOCK: Self = Self {
        outer: DisplayOuter::Inline,
        inner: DisplayInner::FlowRoot,
    };
    pub const FLEX: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::Flex,
    };
    pub const INLINE_FLEX: Self = Self {
        outer: DisplayOuter::Inline,
        inner: DisplayInner::Flex,
    };
    pub const GRID: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::Grid,
    };
    pub const INLINE_GRID: Self = Self {
        outer: DisplayOuter::Inline,
        inner: DisplayInner::Grid,
    };
    pub const TABLE: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::Table,
    };
    pub const TABLE_ROW: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableRow,
    };
    pub const TABLE_CELL: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableCell,
    };
    pub const TABLE_ROW_GROUP: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableRowGroup,
    };
    pub const TABLE_COLUMN: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableColumn,
    };
    pub const TABLE_COLUMN_GROUP: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableColumnGroup,
    };
    pub const TABLE_CAPTION: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableCaption,
    };
    pub const TABLE_HEADER_GROUP: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableHeaderGroup,
    };
    pub const TABLE_FOOTER_GROUP: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::TableFooterGroup,
    };
    pub const NONE: Self = Self {
        outer: DisplayOuter::None,
        inner: DisplayInner::Flow,
    };
    pub const FLOW_ROOT: Self = Self {
        outer: DisplayOuter::Block,
        inner: DisplayInner::FlowRoot,
    };

    pub fn is_none(&self) -> bool {
        self.outer == DisplayOuter::None
    }

    pub fn is_block_level(&self) -> bool {
        self.outer == DisplayOuter::Block
    }

    pub fn is_inline_level(&self) -> bool {
        self.outer == DisplayOuter::Inline
    }

    pub fn establishes_bfc(&self) -> bool {
        matches!(
            self.inner,
            DisplayInner::FlowRoot | DisplayInner::Flex | DisplayInner::Grid | DisplayInner::Table
        )
    }

    pub fn is_table_part(&self) -> bool {
        matches!(
            self.inner,
            DisplayInner::Table
                | DisplayInner::TableRow
                | DisplayInner::TableCell
                | DisplayInner::TableRowGroup
                | DisplayInner::TableColumn
                | DisplayInner::TableColumnGroup
                | DisplayInner::TableCaption
                | DisplayInner::TableHeaderGroup
                | DisplayInner::TableFooterGroup
        )
    }
}

impl Default for Display {
    fn default() -> Self {
        Self::INLINE
    }
}

/// CSS `position` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl Position {
    pub fn is_positioned(&self) -> bool {
        !matches!(self, Position::Static)
    }

    pub fn is_absolutely_positioned(&self) -> bool {
        matches!(self, Position::Absolute | Position::Fixed)
    }

    pub fn is_in_flow(&self) -> bool {
        matches!(
            self,
            Position::Static | Position::Relative | Position::Sticky
        )
    }
}

/// CSS `box-sizing` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

/// CSS `float` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Float {
    #[default]
    None,
    Left,
    Right,
}

/// CSS `clear` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Clear {
    #[default]
    None,
    Left,
    Right,
    Both,
}

/// CSS `overflow` property (per axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
    Auto,
}

/// CSS `text-align` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Right,
    Center,
    Justify,
}

/// CSS `vertical-align` (simplified for inline layout).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum VerticalAlign {
    #[default]
    Baseline,
    Top,
    Middle,
    Bottom,
    Length(f32),
}

/// CSS `white-space` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhiteSpace {
    #[default]
    Normal,
    Nowrap,
    Pre,
    PreWrap,
    PreLine,
}

impl WhiteSpace {
    pub fn preserves_newlines(&self) -> bool {
        matches!(
            self,
            WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine
        )
    }

    pub fn collapses_spaces(&self) -> bool {
        matches!(
            self,
            WhiteSpace::Normal | WhiteSpace::Nowrap | WhiteSpace::PreLine
        )
    }

    pub fn wraps(&self) -> bool {
        matches!(
            self,
            WhiteSpace::Normal | WhiteSpace::PreWrap | WhiteSpace::PreLine
        )
    }
}

// --- Flexbox properties ---

/// CSS `flex-direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    pub fn is_row(&self) -> bool {
        matches!(self, FlexDirection::Row | FlexDirection::RowReverse)
    }

    pub fn is_column(&self) -> bool {
        !self.is_row()
    }

    pub fn is_reverse(&self) -> bool {
        matches!(
            self,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        )
    }
}

/// CSS `flex-wrap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexWrap {
    #[default]
    Nowrap,
    Wrap,
    WrapReverse,
}

/// CSS alignment values (for justify-content, align-items, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Start,
    End,
}

/// CSS `align-self`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Start,
    End,
}

/// CSS `justify-content`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Start,
    End,
}

/// CSS `align-content`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignContent {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Start,
    End,
}

// --- Grid properties ---

/// A grid track sizing function.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TrackSizingFunction {
    /// Fixed length.
    Length(f32),
    /// Percentage of grid container.
    Percentage(f32),
    /// Fraction of remaining space.
    Fr(f32),
    /// Minimum content size.
    MinContent,
    /// Maximum content size.
    MaxContent,
    /// Auto sizing.
    #[default]
    Auto,
    /// minmax(min, max).
    MinMax(Box<TrackSizingFunction>, Box<TrackSizingFunction>),
    /// fit-content(limit).
    FitContent(f32),
}

/// A grid track definition.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackDefinition {
    pub sizing: TrackSizingFunction,
    pub line_name: Option<String>,
}

impl TrackDefinition {
    pub fn new(sizing: TrackSizingFunction) -> Self {
        Self {
            sizing,
            line_name: None,
        }
    }
}

/// Grid auto-flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

/// Grid placement value for a single edge.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum GridPlacement {
    #[default]
    Auto,
    Line(i32),
    Span(u32),
    Named(String),
}

// --- Table properties ---

/// CSS `table-layout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableLayout {
    #[default]
    Auto,
    Fixed,
}

/// CSS `border-collapse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderCollapse {
    #[default]
    Separate,
    Collapse,
}

/// CSS `caption-side`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptionSide {
    #[default]
    Top,
    Bottom,
}

// --- Computed style ---

/// Full computed style for a layout node.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    // Display & positioning
    pub display: Display,
    pub position: Position,
    pub float: Float,
    pub clear: Clear,

    // Box model
    pub box_sizing: BoxSizing,
    pub width: LengthPercentageAuto,
    pub height: LengthPercentageAuto,
    pub min_width: LengthPercentage,
    pub min_height: LengthPercentage,
    pub max_width: LengthPercentageNone,
    pub max_height: LengthPercentageNone,

    pub margin_top: LengthPercentageAuto,
    pub margin_right: LengthPercentageAuto,
    pub margin_bottom: LengthPercentageAuto,
    pub margin_left: LengthPercentageAuto,

    pub padding_top: LengthPercentage,
    pub padding_right: LengthPercentage,
    pub padding_bottom: LengthPercentage,
    pub padding_left: LengthPercentage,

    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,

    // Positioning offsets
    pub top: LengthPercentageAuto,
    pub right: LengthPercentageAuto,
    pub bottom: LengthPercentageAuto,
    pub left: LengthPercentageAuto,

    // Overflow
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,

    // Text/inline
    pub text_align: TextAlign,
    pub vertical_align: VerticalAlign,
    pub line_height: f32,
    pub white_space: WhiteSpace,

    // Flexbox
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: LengthPercentageAuto,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub align_content: AlignContent,
    pub justify_content: JustifyContent,
    pub order: i32,

    // Grid container
    pub grid_template_rows: Vec<TrackDefinition>,
    pub grid_template_columns: Vec<TrackDefinition>,
    pub grid_auto_rows: Vec<TrackSizingFunction>,
    pub grid_auto_columns: Vec<TrackSizingFunction>,
    pub grid_auto_flow: GridAutoFlow,
    pub row_gap: f32,
    pub column_gap: f32,

    // Grid item
    pub grid_row_start: GridPlacement,
    pub grid_row_end: GridPlacement,
    pub grid_column_start: GridPlacement,
    pub grid_column_end: GridPlacement,

    // Table
    pub table_layout: TableLayout,
    pub border_collapse: BorderCollapse,
    pub border_spacing: f32,
    pub caption_side: CaptionSide,

    // Z-index
    pub z_index: crate::values::NumberOrAuto,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::INLINE,
            position: Position::Static,
            float: Float::None,
            clear: Clear::None,

            box_sizing: BoxSizing::ContentBox,
            width: LengthPercentageAuto::Auto,
            height: LengthPercentageAuto::Auto,
            min_width: LengthPercentage::Length(0.0),
            min_height: LengthPercentage::Length(0.0),
            max_width: LengthPercentageNone::None,
            max_height: LengthPercentageNone::None,

            margin_top: LengthPercentageAuto::px(0.0),
            margin_right: LengthPercentageAuto::px(0.0),
            margin_bottom: LengthPercentageAuto::px(0.0),
            margin_left: LengthPercentageAuto::px(0.0),

            padding_top: LengthPercentage::Length(0.0),
            padding_right: LengthPercentage::Length(0.0),
            padding_bottom: LengthPercentage::Length(0.0),
            padding_left: LengthPercentage::Length(0.0),

            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,

            top: LengthPercentageAuto::Auto,
            right: LengthPercentageAuto::Auto,
            bottom: LengthPercentageAuto::Auto,
            left: LengthPercentageAuto::Auto,

            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,

            text_align: TextAlign::Left,
            vertical_align: VerticalAlign::Baseline,
            line_height: 1.2,
            white_space: WhiteSpace::Normal,

            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Nowrap,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: LengthPercentageAuto::Auto,
            align_items: AlignItems::Stretch,
            align_self: AlignSelf::Auto,
            align_content: AlignContent::Stretch,
            justify_content: JustifyContent::FlexStart,
            order: 0,

            grid_template_rows: Vec::new(),
            grid_template_columns: Vec::new(),
            grid_auto_rows: Vec::new(),
            grid_auto_columns: Vec::new(),
            grid_auto_flow: GridAutoFlow::Row,
            row_gap: 0.0,
            column_gap: 0.0,

            grid_row_start: GridPlacement::Auto,
            grid_row_end: GridPlacement::Auto,
            grid_column_start: GridPlacement::Auto,
            grid_column_end: GridPlacement::Auto,

            table_layout: TableLayout::Auto,
            border_collapse: BorderCollapse::Separate,
            border_spacing: 0.0,
            caption_side: CaptionSide::Top,

            z_index: crate::values::NumberOrAuto::Auto,
        }
    }
}

impl ComputedStyle {
    /// Create a block-level element style with default values.
    pub fn block() -> Self {
        Self {
            display: Display::BLOCK,
            ..Default::default()
        }
    }

    /// Create an inline element style.
    pub fn inline() -> Self {
        Self::default()
    }

    /// Whether this element establishes a new block formatting context.
    pub fn establishes_bfc(&self) -> bool {
        self.display.establishes_bfc()
            || self.overflow_x != Overflow::Visible
            || self.overflow_y != Overflow::Visible
            || self.float != Float::None
            || self.position.is_absolutely_positioned()
    }

    /// Whether this element is out of flow.
    pub fn is_out_of_flow(&self) -> bool {
        self.position.is_absolutely_positioned() || self.float != Float::None
    }
}
