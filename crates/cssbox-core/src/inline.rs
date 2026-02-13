//! Inline formatting context implementation.
//!
//! This module handles the layout of inline-level content, including text runs,
//! inline boxes, and line breaking.

use crate::float::FloatContext;
use crate::fragment::{Fragment, FragmentKind};
use crate::geometry::{Point, Size};
use crate::layout::LayoutContext;
use crate::style::{TextAlign, VerticalAlign, WhiteSpace};
use crate::tree::NodeId;

/// An inline item that needs to be laid out within a line.
#[derive(Debug, Clone)]
struct InlineItem {
    node: NodeId,
    size: Size,
    baseline: f32,
}

/// A single line box containing inline items.
#[derive(Debug)]
struct LineBox {
    items: Vec<InlineItem>,
    width: f32,
    height: f32,
    baseline: f32,
}

impl LineBox {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            width: 0.0,
            height: 0.0,
            baseline: 0.0,
        }
    }

    fn add_item(&mut self, item: InlineItem) {
        self.width += item.size.width;
        self.height = self.height.max(item.size.height);
        self.baseline = self.baseline.max(item.baseline);
        self.items.push(item);
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn used_width(&self) -> f32 {
        self.width
    }
}

/// Layout an inline formatting context.
///
/// This function creates line boxes from inline-level children of the parent node,
/// applying line breaking, text alignment, and vertical alignment.
///
/// # Arguments
///
/// * `ctx` - The layout context with tree and text measurement
/// * `parent` - The parent node establishing the inline formatting context
/// * `containing_width` - The available width for lines
/// * `fragments` - Output vector to append generated fragments
/// * `float_ctx` - Float context for handling floated elements (currently unused)
///
/// # Returns
///
/// The total height consumed by all line boxes.
pub fn layout_inline_formatting_context(
    ctx: &LayoutContext,
    parent: NodeId,
    containing_width: f32,
    fragments: &mut Vec<Fragment>,
    _float_ctx: &mut FloatContext,
) -> f32 {
    let parent_style = ctx.tree.style(parent);
    let text_align = parent_style.text_align;
    let line_height = parent_style.line_height;
    let white_space = parent_style.white_space;

    // Collect inline items from children
    let mut inline_items = Vec::new();
    collect_inline_items(ctx, parent, containing_width, &mut inline_items);

    // Break into line boxes
    let mut line_boxes = Vec::new();
    break_into_lines(
        &inline_items,
        containing_width,
        white_space,
        &mut line_boxes,
    );

    // Position line boxes and create fragments
    let mut current_y = 0.0;

    for line_box in &line_boxes {
        let line_height_actual = line_box.height.max(line_height);

        // Apply text alignment
        let line_offset_x = calculate_text_align_offset(
            text_align,
            containing_width,
            line_box.used_width(),
            line_box.items.len(),
        );

        // Create line box fragment
        let mut line_fragment = Fragment::new(parent, FragmentKind::LineBox);
        line_fragment.position = Point::new(0.0, current_y);
        line_fragment.size = Size::new(containing_width, line_height_actual);

        // Position items within line
        let mut current_x = line_offset_x;

        for (idx, item) in line_box.items.iter().enumerate() {
            let mut item_fragment = Fragment::new(item.node, FragmentKind::TextRun);
            item_fragment.size = item.size;

            // Calculate vertical position based on vertical-align
            let item_style = ctx.tree.style(item.node);
            let vertical_offset = calculate_vertical_align_offset(
                item_style.vertical_align,
                line_box.baseline,
                item.baseline,
                item.size.height,
                line_height_actual,
            );

            item_fragment.position = Point::new(current_x, vertical_offset);

            // Apply justify spacing if needed
            let extra_space = if text_align == TextAlign::Justify
                && line_box.items.len() > 1
                && idx < line_box.items.len() - 1
            {
                let remaining = containing_width - line_box.used_width();
                let gaps = (line_box.items.len() - 1) as f32;
                remaining / gaps
            } else {
                0.0
            };

            current_x += item.size.width + extra_space;

            line_fragment.children.push(item_fragment);
        }

        current_y += line_height_actual;
        fragments.push(line_fragment);
    }

    current_y
}

/// Collect inline items from the children of a parent node.
fn collect_inline_items(
    ctx: &LayoutContext,
    parent: NodeId,
    max_width: f32,
    items: &mut Vec<InlineItem>,
) {
    let children = ctx.tree.children(parent);

    for &child in children {
        let node = ctx.tree.node(child);
        let style = ctx.tree.style(child);

        if node.is_text() {
            if let Some(text) = node.text_content() {
                // Measure text
                let font_size = style.line_height / 1.2;
                let size = ctx.text_measure.measure(text, font_size, max_width);

                // For text, baseline is typically at bottom minus descent
                // Simplified: baseline is at 80% of height (20% descent)
                let baseline = size.height * 0.8;

                items.push(InlineItem {
                    node: child,
                    size,
                    baseline,
                });
            }
        } else {
            // Inline-level element - recursively collect or measure as atomic
            // For now, treat as atomic inline-block
            // In a full implementation, we'd check display type and recurse if inline
            let font_size = style.line_height / 1.2;
            let size = ctx.text_measure.measure("X", font_size, max_width);
            let baseline = size.height * 0.8;

            items.push(InlineItem {
                node: child,
                size,
                baseline,
            });
        }
    }
}

/// Break inline items into line boxes using greedy line breaking.
fn break_into_lines(
    items: &[InlineItem],
    containing_width: f32,
    white_space: WhiteSpace,
    line_boxes: &mut Vec<LineBox>,
) {
    if items.is_empty() {
        return;
    }

    let allow_wrapping = white_space.wraps();
    let mut current_line = LineBox::new();

    for item in items {
        let item_width = item.size.width;

        // Check if item fits on current line
        let fits =
            current_line.is_empty() || current_line.used_width() + item_width <= containing_width;

        if fits || !allow_wrapping {
            // Add to current line
            current_line.add_item(item.clone());
        } else {
            // Start new line
            if !current_line.is_empty() {
                line_boxes.push(current_line);
            }
            current_line = LineBox::new();
            current_line.add_item(item.clone());
        }
    }

    // Push final line
    if !current_line.is_empty() {
        line_boxes.push(current_line);
    }
}

/// Calculate horizontal offset for text alignment.
fn calculate_text_align_offset(
    align: TextAlign,
    containing_width: f32,
    line_width: f32,
    _item_count: usize,
) -> f32 {
    match align {
        TextAlign::Left => 0.0,
        TextAlign::Right => (containing_width - line_width).max(0.0),
        TextAlign::Center => ((containing_width - line_width) / 2.0).max(0.0),
        TextAlign::Justify => {
            // Justify only applies to spacing between items, not initial offset
            0.0
        }
    }
}

/// Calculate vertical offset for an item based on vertical-align.
fn calculate_vertical_align_offset(
    align: VerticalAlign,
    line_baseline: f32,
    item_baseline: f32,
    item_height: f32,
    line_height: f32,
) -> f32 {
    match align {
        VerticalAlign::Baseline => line_baseline - item_baseline,
        VerticalAlign::Top => 0.0,
        VerticalAlign::Bottom => line_height - item_height,
        VerticalAlign::Middle => (line_height - item_height) / 2.0,
        VerticalAlign::Length(offset) => line_baseline - item_baseline + offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::FixedWidthTextMeasure;
    use crate::style::ComputedStyle;
    use crate::tree::{BoxTreeBuilder, NodeKind, TextContent};

    #[test]
    fn test_single_line_layout() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());
        builder.text(root, "Hello");
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(800.0);
        let height =
            layout_inline_formatting_context(&ctx, root, 800.0, &mut fragments, &mut float_ctx);

        assert_eq!(fragments.len(), 1); // One line box
        assert!(height > 0.0);
        assert_eq!(fragments[0].kind, FragmentKind::LineBox);
        assert_eq!(fragments[0].children.len(), 1); // One text run
    }

    #[test]
    fn test_line_wrapping() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());
        builder.text(root, "Hello");
        builder.text(root, "World");
        builder.text(root, "Test");
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(50.0);

        // Narrow width to force wrapping
        let height =
            layout_inline_formatting_context(&ctx, root, 50.0, &mut fragments, &mut float_ctx);

        // Should have multiple lines due to narrow width
        // Default line_height is 1.2 (unitless), so each line is 1.2px tall
        // With 3 text items wrapping to 3 lines: total height = 3.6px
        assert!(fragments.len() >= 2);
        assert!(height > 1.2); // More than one line height
    }

    #[test]
    fn test_text_align_center() {
        let mut builder = BoxTreeBuilder::new();
        let mut style = ComputedStyle::block();
        style.text_align = TextAlign::Center;
        let root = builder.root(style);
        builder.text(root, "Hi");
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(800.0);
        layout_inline_formatting_context(&ctx, root, 800.0, &mut fragments, &mut float_ctx);

        // Check that text is not at x=0 (it's centered)
        let line = &fragments[0];
        let text_run = &line.children[0];
        // "Hi" = 2 chars * 8px = 16px, centered in 800px = (800-16)/2 = 392
        assert!((text_run.position.x - 392.0).abs() < 1.0);
    }

    #[test]
    fn test_text_align_right() {
        let mut builder = BoxTreeBuilder::new();
        let mut style = ComputedStyle::block();
        style.text_align = TextAlign::Right;
        let root = builder.root(style);
        builder.text(root, "Hi");
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(800.0);
        layout_inline_formatting_context(&ctx, root, 800.0, &mut fragments, &mut float_ctx);

        // "Hi" = 2 chars * 8px = 16px, right-aligned in 800px = 800 - 16 = 784
        let line = &fragments[0];
        let text_run = &line.children[0];
        assert!((text_run.position.x - 784.0).abs() < 1.0);
    }

    #[test]
    fn test_text_align_justify() {
        let mut builder = BoxTreeBuilder::new();
        let mut style = ComputedStyle::block();
        style.text_align = TextAlign::Justify;
        let root = builder.root(style);
        builder.text(root, "A");
        builder.text(root, "B");
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(100.0);
        layout_inline_formatting_context(&ctx, root, 100.0, &mut fragments, &mut float_ctx);

        let line = &fragments[0];
        assert_eq!(line.children.len(), 2);

        // First item at x=0
        assert_eq!(line.children[0].position.x, 0.0);

        // Second item should have extra space added for justification
        // Each text is 8px wide, total 16px in 100px line
        // Extra space = 100 - 16 = 84px distributed across 1 gap
        // Second item x = 8 + 84 = 92
        assert!((line.children[1].position.x - 92.0).abs() < 1.0);
    }

    #[test]
    fn test_vertical_align_baseline() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());
        builder.text(root, "Test");
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(800.0);
        layout_inline_formatting_context(&ctx, root, 800.0, &mut fragments, &mut float_ctx);

        // Default is baseline alignment
        let line = &fragments[0];
        let text_run = &line.children[0];

        // Vertical offset should position text on baseline
        assert!(text_run.position.y >= 0.0);
    }

    #[test]
    fn test_vertical_align_top() {
        let mut builder = BoxTreeBuilder::new();
        let style = ComputedStyle::block();
        let root = builder.root(style);

        let mut text_style = ComputedStyle::inline();
        text_style.vertical_align = VerticalAlign::Top;

        // Create text with custom style
        let text_id = builder.tree.add_node(
            NodeKind::Text(TextContent {
                text: "Test".to_string(),
            }),
            text_style,
        );
        builder.tree.append_child(root, text_id);

        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(800.0);
        layout_inline_formatting_context(&ctx, root, 800.0, &mut fragments, &mut float_ctx);

        let line = &fragments[0];
        let text_run = &line.children[0];

        // Top alignment means y = 0
        assert_eq!(text_run.position.y, 0.0);
    }

    #[test]
    fn test_empty_inline_context() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(800.0);
        let height =
            layout_inline_formatting_context(&ctx, root, 800.0, &mut fragments, &mut float_ctx);

        assert_eq!(height, 0.0);
        assert_eq!(fragments.len(), 0);
    }

    #[test]
    fn test_white_space_nowrap() {
        let mut builder = BoxTreeBuilder::new();
        let mut style = ComputedStyle::block();
        style.white_space = WhiteSpace::Nowrap;
        let root = builder.root(style);
        builder.text(root, "A");
        builder.text(root, "B");
        builder.text(root, "C");
        let tree = builder.build();

        let ctx = LayoutContext {
            tree: &tree,
            text_measure: &FixedWidthTextMeasure,
            viewport: Size::new(800.0, 600.0),
        };

        let mut fragments = Vec::new();
        let mut float_ctx = FloatContext::new(10.0);

        // Even with narrow width, nowrap should keep everything on one line
        layout_inline_formatting_context(&ctx, root, 10.0, &mut fragments, &mut float_ctx);

        // Should be one line despite narrow width
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].children.len(), 3);
    }
}
