//! Box model resolution: margin, border, padding computation.

use crate::geometry::Edges;
use crate::style::ComputedStyle;

/// Resolved box model dimensions for a layout box.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxModel {
    pub margin: Edges,
    pub border: Edges,
    pub padding: Edges,
}

impl BoxModel {
    /// Compute the border edges from a computed style.
    pub fn resolve_border(style: &ComputedStyle) -> Edges {
        Edges::new(
            style.border_top_width,
            style.border_right_width,
            style.border_bottom_width,
            style.border_left_width,
        )
    }

    /// Compute padding edges, resolving percentages against containing block width.
    pub fn resolve_padding(style: &ComputedStyle, containing_block_width: f32) -> Edges {
        // CSS 2.1 §8.4: padding percentages resolve against containing block width,
        // even for vertical padding.
        Edges::new(
            style.padding_top.resolve(containing_block_width),
            style.padding_right.resolve(containing_block_width),
            style.padding_bottom.resolve(containing_block_width),
            style.padding_left.resolve(containing_block_width),
        )
    }

    /// Compute margin edges, resolving percentages against containing block width.
    /// Auto margins return 0.0 here — auto margin resolution happens during layout.
    pub fn resolve_margin(style: &ComputedStyle, containing_block_width: f32) -> Edges {
        // CSS 2.1 §8.3: margin percentages resolve against containing block width,
        // even for vertical margins.
        Edges::new(
            style
                .margin_top
                .resolve(containing_block_width)
                .unwrap_or(0.0),
            style
                .margin_right
                .resolve(containing_block_width)
                .unwrap_or(0.0),
            style
                .margin_bottom
                .resolve(containing_block_width)
                .unwrap_or(0.0),
            style
                .margin_left
                .resolve(containing_block_width)
                .unwrap_or(0.0),
        )
    }

    /// Resolve all box model dimensions.
    pub fn resolve(style: &ComputedStyle, containing_block_width: f32) -> Self {
        Self {
            margin: Self::resolve_margin(style, containing_block_width),
            border: Self::resolve_border(style),
            padding: Self::resolve_padding(style, containing_block_width),
        }
    }

    /// Total horizontal space consumed by margin + border + padding.
    pub fn horizontal_total(&self) -> f32 {
        self.margin.horizontal() + self.border.horizontal() + self.padding.horizontal()
    }

    /// Total vertical space consumed by margin + border + padding.
    pub fn vertical_total(&self) -> f32 {
        self.margin.vertical() + self.border.vertical() + self.padding.vertical()
    }

    /// Horizontal space consumed by border + padding (no margin).
    pub fn horizontal_border_padding(&self) -> f32 {
        self.border.horizontal() + self.padding.horizontal()
    }

    /// Vertical space consumed by border + padding (no margin).
    pub fn vertical_border_padding(&self) -> f32 {
        self.border.vertical() + self.padding.vertical()
    }
}

/// Resolve the content width of a block-level box per CSS 2.1 §10.3.3.
///
/// The constraint equation for block-level non-replaced elements in normal flow:
/// margin-left + border-left + padding-left + width + padding-right + border-right + margin-right
///   = containing block width
pub fn resolve_block_width(style: &ComputedStyle, containing_block_width: f32) -> (f32, Edges) {
    let border = BoxModel::resolve_border(style);
    let padding = BoxModel::resolve_padding(style, containing_block_width);

    let border_padding_h = border.horizontal() + padding.horizontal();

    // Resolve specified width
    let specified_width = style.width.resolve(containing_block_width);

    // Resolve specified margins
    let margin_left_specified = style.margin_left.resolve(containing_block_width);
    let margin_right_specified = style.margin_right.resolve(containing_block_width);

    let (content_width, margin_left, margin_right) = match specified_width {
        Some(mut w) => {
            // If box-sizing: border-box, width includes border + padding
            if style.box_sizing == crate::style::BoxSizing::BorderBox {
                w = (w - border_padding_h).max(0.0);
            }

            // Apply min/max constraints
            let min_w = style.min_width.resolve(containing_block_width);
            let max_w = style
                .max_width
                .resolve(containing_block_width)
                .unwrap_or(f32::INFINITY);
            w = w.max(min_w).min(max_w);

            let remaining = containing_block_width - w - border_padding_h;

            match (margin_left_specified, margin_right_specified) {
                (Some(ml), Some(mr)) => {
                    // Over-constrained: adjust margin-right (LTR)
                    let _total = ml + mr;
                    let actual_mr = remaining - ml;
                    (w, ml, actual_mr)
                }
                (None, Some(mr)) => {
                    let ml = remaining - mr;
                    (w, ml, mr)
                }
                (Some(ml), None) => {
                    let mr = remaining - ml;
                    (w, ml, mr)
                }
                (None, None) => {
                    // Both auto: split remaining space equally
                    let each = remaining / 2.0;
                    (w, each, each)
                }
            }
        }
        None => {
            // Width is auto: fill available space
            let ml = margin_left_specified.unwrap_or(0.0);
            let mr = margin_right_specified.unwrap_or(0.0);
            let mut w = containing_block_width - border_padding_h - ml - mr;

            // Apply min/max constraints
            let min_w = style.min_width.resolve(containing_block_width);
            let max_w = style
                .max_width
                .resolve(containing_block_width)
                .unwrap_or(f32::INFINITY);
            w = w.max(min_w).min(max_w);

            (w.max(0.0), ml, mr)
        }
    };

    let margin = Edges::new(
        style
            .margin_top
            .resolve(containing_block_width)
            .unwrap_or(0.0),
        margin_right,
        style
            .margin_bottom
            .resolve(containing_block_width)
            .unwrap_or(0.0),
        margin_left,
    );

    (content_width, margin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::ComputedStyle;
    use crate::values::LengthPercentageAuto;

    #[test]
    fn test_block_width_auto_fills_container() {
        let style = ComputedStyle::block();
        let (width, margin) = resolve_block_width(&style, 800.0);
        assert_eq!(width, 800.0);
        assert_eq!(margin.left, 0.0);
        assert_eq!(margin.right, 0.0);
    }

    #[test]
    fn test_block_width_fixed_centers_with_auto_margins() {
        let mut style = ComputedStyle::block();
        style.width = LengthPercentageAuto::px(400.0);
        style.margin_left = LengthPercentageAuto::Auto;
        style.margin_right = LengthPercentageAuto::Auto;
        let (width, margin) = resolve_block_width(&style, 800.0);
        assert_eq!(width, 400.0);
        assert_eq!(margin.left, 200.0);
        assert_eq!(margin.right, 200.0);
    }

    #[test]
    fn test_block_width_with_padding() {
        let mut style = ComputedStyle::block();
        style.padding_left = crate::values::LengthPercentage::px(20.0);
        style.padding_right = crate::values::LengthPercentage::px(20.0);
        let (width, _margin) = resolve_block_width(&style, 800.0);
        assert_eq!(width, 760.0); // 800 - 20 - 20
    }

    #[test]
    fn test_block_width_border_box() {
        let mut style = ComputedStyle::block();
        style.width = LengthPercentageAuto::px(400.0);
        style.box_sizing = crate::style::BoxSizing::BorderBox;
        style.padding_left = crate::values::LengthPercentage::px(20.0);
        style.padding_right = crate::values::LengthPercentage::px(20.0);
        style.border_left_width = 5.0;
        style.border_right_width = 5.0;
        let (width, _margin) = resolve_block_width(&style, 800.0);
        assert_eq!(width, 350.0); // 400 - 20 - 20 - 5 - 5
    }
}
