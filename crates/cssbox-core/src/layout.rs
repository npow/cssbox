//! Layout entry point and dispatch.

use crate::block;
use crate::flex;
use crate::fragment::{Fragment, FragmentKind, LayoutResult};
use crate::geometry::Size;
use crate::grid;
use crate::position;
use crate::table;
use crate::tree::{BoxTree, FormattingContextType, NodeId};

/// Text measurement callback.
pub trait TextMeasure {
    /// Measure the width and height of a text string within a maximum width.
    fn measure(&self, text: &str, font_size: f32, max_width: f32) -> Size;
}

/// A fixed-width text measurer for testing (every character = 8px wide).
pub struct FixedWidthTextMeasure;

impl TextMeasure for FixedWidthTextMeasure {
    fn measure(&self, text: &str, font_size: f32, max_width: f32) -> Size {
        let char_width = 8.0;
        let line_height = font_size * 1.2;

        if text.is_empty() {
            return Size::new(0.0, line_height);
        }

        let chars_per_line = (max_width / char_width).floor().max(1.0) as usize;
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            return Size::new(0.0, line_height);
        }

        let mut lines = 1usize;
        let mut current_line_chars = 0usize;
        let mut max_line_width = 0.0f32;

        for word in words.iter() {
            let word_chars = word.len();
            let needed = if current_line_chars > 0 {
                word_chars + 1 // space before word
            } else {
                word_chars
            };

            if current_line_chars > 0 && current_line_chars + needed > chars_per_line {
                // Wrap to new line
                let line_width = current_line_chars as f32 * char_width;
                max_line_width = max_line_width.max(line_width);
                lines += 1;
                current_line_chars = word_chars;
            } else {
                if current_line_chars > 0 {
                    current_line_chars += 1; // space
                }
                current_line_chars += word_chars;
            }
        }

        // Final line
        let line_width = current_line_chars as f32 * char_width;
        max_line_width = max_line_width.max(line_width);

        Size::new(max_line_width, lines as f32 * line_height)
    }
}

/// Layout context passed through the layout tree.
pub struct LayoutContext<'a> {
    pub tree: &'a BoxTree,
    pub text_measure: &'a dyn TextMeasure,
    pub viewport: Size,
}

/// Compute layout for an entire box tree.
pub fn compute_layout(
    tree: &BoxTree,
    text_measure: &dyn TextMeasure,
    viewport: Size,
) -> LayoutResult {
    let ctx = LayoutContext {
        tree,
        text_measure,
        viewport,
    };

    let root = tree.root();
    let root_fragment = layout_node(&ctx, root, viewport.width, viewport.height);

    // After main layout, resolve absolute/fixed positioned elements
    let root_fragment = position::resolve_positioned(tree, root_fragment, viewport);

    LayoutResult {
        root: root_fragment,
    }
}

/// Layout a single node and its subtree.
pub fn layout_node(
    ctx: &LayoutContext,
    node: NodeId,
    containing_block_width: f32,
    containing_block_height: f32,
) -> Fragment {
    let style = ctx.tree.style(node);

    // Skip display: none
    if style.display.is_none() {
        let mut f = Fragment::new(node, FragmentKind::Box);
        f.size = Size::ZERO;
        return f;
    }

    // Text nodes
    if let Some(text) = ctx.tree.node(node).text_content() {
        return layout_text(ctx, node, text, containing_block_width);
    }

    // Determine formatting context for children
    let fc = ctx.tree.formatting_context(node);

    match fc {
        FormattingContextType::Block => {
            block::layout_block(ctx, node, containing_block_width, containing_block_height)
        }
        FormattingContextType::Inline => {
            block::layout_block(ctx, node, containing_block_width, containing_block_height)
        }
        FormattingContextType::Flex => {
            flex::layout_flex(ctx, node, containing_block_width, containing_block_height)
        }
        FormattingContextType::Grid => {
            grid::layout_grid(ctx, node, containing_block_width, containing_block_height)
        }
        FormattingContextType::Table => {
            table::layout_table(ctx, node, containing_block_width, containing_block_height)
        }
    }
}

/// Layout a text node.
fn layout_text(ctx: &LayoutContext, node: NodeId, text: &str, max_width: f32) -> Fragment {
    let style = ctx.tree.style(node);
    let font_size = style.line_height / 1.2; // approximate
    let size = ctx.text_measure.measure(text, font_size, max_width);

    let mut fragment = Fragment::new(node, FragmentKind::TextRun);
    fragment.size = size;
    fragment
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::ComputedStyle;
    use crate::tree::BoxTreeBuilder;
    use crate::values::LengthPercentageAuto;

    #[test]
    fn test_simple_block_layout() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());
        let mut child_style = ComputedStyle::block();
        child_style.height = LengthPercentageAuto::px(100.0);
        builder.element(root, child_style);

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let root_rect = result.bounding_rect(tree.root()).unwrap();
        assert_eq!(root_rect.width, 800.0);
        assert_eq!(root_rect.height, 100.0);
    }

    #[test]
    fn test_two_blocks_stack_vertically() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(ComputedStyle::block());

        let mut child1_style = ComputedStyle::block();
        child1_style.height = LengthPercentageAuto::px(50.0);
        let c1 = builder.element(root, child1_style);

        let mut child2_style = ComputedStyle::block();
        child2_style.height = LengthPercentageAuto::px(75.0);
        let c2 = builder.element(root, child2_style);

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let root_rect = result.bounding_rect(tree.root()).unwrap();
        assert_eq!(root_rect.height, 125.0); // 50 + 75

        let c1_rect = result.bounding_rect(c1).unwrap();
        assert_eq!(c1_rect.y, 0.0);
        assert_eq!(c1_rect.height, 50.0);

        let c2_rect = result.bounding_rect(c2).unwrap();
        assert_eq!(c2_rect.y, 50.0);
        assert_eq!(c2_rect.height, 75.0);
    }

    #[test]
    fn test_fixed_width_text_measure() {
        let m = FixedWidthTextMeasure;
        let size = m.measure("Hello World", 16.0, 800.0);
        assert_eq!(size.width, 88.0); // 11 chars * 8px
        assert!((size.height - 19.2).abs() < 0.01); // 16 * 1.2
    }
}
