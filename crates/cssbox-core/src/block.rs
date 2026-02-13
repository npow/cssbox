//! Block formatting context layout.
//!
//! Implements CSS 2.1 §9.4.1 — Block Formatting Contexts and
//! CSS 2.1 §10.3.3 — Block-level non-replaced elements in normal flow.

use crate::box_model::{resolve_block_width, BoxModel};
use crate::float::FloatContext;
use crate::fragment::{Fragment, FragmentKind};
use crate::geometry::{Point, Size};
use crate::inline;
use crate::layout::{self, LayoutContext};
use crate::style::{BoxSizing, Float};
use crate::tree::{FormattingContextType, NodeId};

/// Layout a block-level element and its children.
pub fn layout_block(
    ctx: &LayoutContext,
    node: NodeId,
    containing_block_width: f32,
    containing_block_height: f32,
) -> Fragment {
    let style = ctx.tree.style(node);
    let mut fragment = Fragment::new(node, FragmentKind::Box);

    // 1. Resolve width and margins
    let (content_width, margin) = resolve_block_width(style, containing_block_width);

    // 2. Resolve border and padding
    let border = BoxModel::resolve_border(style);
    let padding = BoxModel::resolve_padding(style, containing_block_width);

    fragment.margin = margin;
    fragment.border = border;
    fragment.padding = padding;

    // 3. Layout children
    let fc_type = ctx.tree.formatting_context(node);

    let mut float_ctx = FloatContext::new(content_width);

    let content_height = match fc_type {
        FormattingContextType::Inline => layout_inline_children(
            ctx,
            node,
            content_width,
            &mut fragment.children,
            &mut float_ctx,
        ),
        _ => layout_block_children(
            ctx,
            node,
            content_width,
            containing_block_height,
            &mut fragment.children,
            &mut float_ctx,
        ),
    };

    // 4. Resolve height
    let specified_height = style.height.resolve(containing_block_height);
    let mut final_height = match specified_height {
        Some(mut h) => {
            if style.box_sizing == BoxSizing::BorderBox {
                h = (h - border.vertical() - padding.vertical()).max(0.0);
            }
            h
        }
        None => content_height,
    };

    // Apply min/max height
    let min_h = style.min_height.resolve(containing_block_height);
    let max_h = style
        .max_height
        .resolve(containing_block_height)
        .unwrap_or(f32::INFINITY);
    final_height = final_height.max(min_h).min(max_h);

    // If this block establishes a BFC, it must contain floats
    if style.establishes_bfc() {
        final_height = final_height.max(float_ctx.clear_all());
    }

    fragment.size = Size::new(content_width, final_height);

    fragment
}

/// Layout block-level children within a block formatting context.
fn layout_block_children(
    ctx: &LayoutContext,
    parent: NodeId,
    containing_width: f32,
    containing_height: f32,
    fragments: &mut Vec<Fragment>,
    float_ctx: &mut FloatContext,
) -> f32 {
    let children = ctx.tree.children(parent);
    let mut cursor_y: f32 = 0.0;
    let mut prev_margin_bottom: f32 = 0.0;
    let mut is_first_child = true;

    for &child_id in children {
        let child_style = ctx.tree.style(child_id);

        // Skip display:none
        if child_style.display.is_none() {
            continue;
        }

        // Handle out-of-flow elements
        if child_style.position.is_absolutely_positioned() {
            // Absolutely positioned elements are laid out later in position::resolve_positioned
            let mut child_fragment =
                layout::layout_node(ctx, child_id, containing_width, containing_height);
            child_fragment.position = Point::ZERO; // placeholder, resolved later
            fragments.push(child_fragment);
            continue;
        }

        // Handle floats
        if child_style.float != Float::None {
            let child_fragment =
                layout::layout_node(ctx, child_id, containing_width, containing_height);
            let floated = float_ctx.place_float(child_fragment, child_style.float, cursor_y);
            fragments.push(floated);
            continue;
        }

        // Handle clear
        if child_style.clear != crate::style::Clear::None {
            let clear_y = float_ctx.clear(child_style.clear);
            cursor_y = cursor_y.max(clear_y);
        }

        // Layout the child
        let mut child_fragment =
            layout::layout_node(ctx, child_id, containing_width, containing_height);

        // Margin collapsing (CSS 2.1 §8.3.1)
        let child_margin_top = child_fragment.margin.top;
        let collapsed_margin = collapse_margins(prev_margin_bottom, child_margin_top);

        if is_first_child {
            // First child: potentially collapse with parent's top margin
            // (simplified — full collapsing is more complex)
            cursor_y += child_margin_top;
        } else {
            // Collapse adjacent sibling margins
            cursor_y -= prev_margin_bottom;
            cursor_y += collapsed_margin;
        }

        // Position the child
        child_fragment.position = Point::new(
            child_fragment.margin.left + child_fragment.border.left + child_fragment.padding.left
                - child_fragment.border.left
                - child_fragment.padding.left,
            cursor_y,
        );
        // Actually, position is the top-left of the border box, including margin offset
        child_fragment.position = Point::new(child_fragment.margin.left, cursor_y);

        // Advance cursor
        cursor_y += child_fragment.border_box().height;
        prev_margin_bottom = child_fragment.margin.bottom;
        is_first_child = false;

        fragments.push(child_fragment);
    }

    // Account for last child's bottom margin (may collapse with parent)
    cursor_y
}

/// Layout inline-level children within a block container.
fn layout_inline_children(
    ctx: &LayoutContext,
    parent: NodeId,
    containing_width: f32,
    fragments: &mut Vec<Fragment>,
    float_ctx: &mut FloatContext,
) -> f32 {
    inline::layout_inline_formatting_context(ctx, parent, containing_width, fragments, float_ctx)
}

/// Collapse two adjacent margins per CSS 2.1 §8.3.1.
///
/// - Both positive: use the larger.
/// - Both negative: use the more negative (larger absolute value).
/// - One of each: sum them (positive + negative).
fn collapse_margins(margin_a: f32, margin_b: f32) -> f32 {
    if margin_a >= 0.0 && margin_b >= 0.0 {
        margin_a.max(margin_b)
    } else if margin_a < 0.0 && margin_b < 0.0 {
        margin_a.min(margin_b)
    } else {
        margin_a + margin_b
    }
}

/// Compute shrink-to-fit width for a block (CSS 2.1 §10.3.5).
/// Used for floats, inline-blocks, absolutely positioned elements, etc.
pub fn shrink_to_fit_width(ctx: &LayoutContext, node: NodeId, available_width: f32) -> f32 {
    // Preferred width: layout with no constraint (max-content)
    let preferred = compute_intrinsic_width(ctx, node, f32::INFINITY);
    // Preferred minimum width: layout with zero width (min-content)
    let preferred_min = compute_intrinsic_width(ctx, node, 0.0);

    // shrink-to-fit = min(max(preferred minimum, available), preferred)
    preferred_min.max(0.0).max(preferred.min(available_width))
}

/// Compute intrinsic width of a node given a constraint.
fn compute_intrinsic_width(ctx: &LayoutContext, node: NodeId, available: f32) -> f32 {
    let children = ctx.tree.children(node);
    let style = ctx.tree.style(node);

    let _border = BoxModel::resolve_border(style);
    let _padding = BoxModel::resolve_padding(style, available);

    let mut max_child_width: f32 = 0.0;

    for &child_id in children {
        let child_style = ctx.tree.style(child_id);
        if child_style.display.is_none() || child_style.is_out_of_flow() {
            continue;
        }

        if let Some(text) = ctx.tree.node(child_id).text_content() {
            let size = ctx.text_measure.measure(text, 16.0, available);
            max_child_width = max_child_width.max(size.width);
        } else if child_style.display.is_block_level() {
            let child_width = if let Some(w) = child_style.width.resolve(available) {
                w
            } else {
                compute_intrinsic_width(ctx, child_id, available)
            };
            let child_box = BoxModel::resolve(child_style, available);
            max_child_width =
                max_child_width.max(child_width + child_box.horizontal_border_padding());
        }
    }

    max_child_width
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{compute_layout, FixedWidthTextMeasure};
    use crate::style::ComputedStyle;
    use crate::tree::BoxTreeBuilder;
    use crate::values::{LengthPercentage, LengthPercentageAuto};

    #[test]
    fn test_margin_collapsing_positive() {
        assert_eq!(collapse_margins(10.0, 20.0), 20.0);
    }

    #[test]
    fn test_margin_collapsing_negative() {
        assert_eq!(collapse_margins(-10.0, -20.0), -20.0);
    }

    #[test]
    fn test_margin_collapsing_mixed() {
        assert_eq!(collapse_margins(10.0, -5.0), 5.0);
    }

    #[test]
    fn test_block_with_padding_and_children() {
        let mut builder = BoxTreeBuilder::new();
        let mut root_style = ComputedStyle::block();
        root_style.padding_top = LengthPercentage::px(10.0);
        root_style.padding_bottom = LengthPercentage::px(10.0);
        let root = builder.root(root_style);

        let mut child_style = ComputedStyle::block();
        child_style.height = LengthPercentageAuto::px(100.0);
        builder.element(root, child_style);

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let root_rect = result.bounding_rect(tree.root()).unwrap();
        // Root: 10px top padding + 100px child + 10px bottom padding = 120px border box height
        assert_eq!(root_rect.height, 120.0);
    }

    #[test]
    fn test_nested_blocks() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());

        let mut outer_style = ComputedStyle::block();
        outer_style.padding_left = LengthPercentage::px(20.0);
        outer_style.padding_right = LengthPercentage::px(20.0);
        let outer = builder.element(root, outer_style);

        let mut inner_style = ComputedStyle::block();
        inner_style.height = LengthPercentageAuto::px(50.0);
        let inner = builder.element(outer, inner_style);

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let inner_rect = result.bounding_rect(inner).unwrap();
        // Inner width: 800 - 20 - 20 = 760
        assert_eq!(inner_rect.width, 760.0);
        // Inner x: outer x (0) + outer border-left (0) + outer padding-left (20)
        assert_eq!(inner_rect.x, 20.0);
    }

    #[test]
    fn test_percentage_width() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());

        let mut child_style = ComputedStyle::block();
        child_style.width = LengthPercentageAuto::percent(50.0);
        child_style.height = LengthPercentageAuto::px(100.0);
        let child = builder.element(root, child_style);

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let child_rect = result.bounding_rect(child).unwrap();
        assert_eq!(child_rect.width, 400.0); // 50% of 800
    }

    #[test]
    fn test_min_max_width() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());

        let mut child_style = ComputedStyle::block();
        child_style.width = LengthPercentageAuto::px(1000.0);
        child_style.max_width = crate::values::LengthPercentageNone::px(500.0);
        child_style.height = LengthPercentageAuto::px(50.0);
        let child = builder.element(root, child_style);

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let child_rect = result.bounding_rect(child).unwrap();
        assert_eq!(child_rect.width, 500.0); // clamped by max-width
    }
}
