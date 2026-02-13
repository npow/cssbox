//! Flexbox layout algorithm.
//!
//! Implements CSS Flexible Box Layout Module Level 1.
//! Reference: https://www.w3.org/TR/css-flexbox-1/#layout-algorithm

use crate::box_model::BoxModel;
use crate::fragment::{Fragment, FragmentKind};
use crate::geometry::{Point, Size};
use crate::layout::{self, LayoutContext};
use crate::style::*;
use crate::tree::NodeId;
use crate::values::LengthPercentageAuto;

/// Layout a flex container and its items.
pub fn layout_flex(
    ctx: &LayoutContext,
    node: NodeId,
    containing_block_width: f32,
    containing_block_height: f32,
) -> Fragment {
    let style = ctx.tree.style(node);
    let mut fragment = Fragment::new(node, FragmentKind::Box);

    // Resolve container box model
    let border = BoxModel::resolve_border(style);
    let padding = BoxModel::resolve_padding(style, containing_block_width);
    let margin = BoxModel::resolve_margin(style, containing_block_width);

    fragment.border = border;
    fragment.padding = padding;
    fragment.margin = margin;

    // Determine container dimensions
    let content_box_width = match style.width.resolve(containing_block_width) {
        Some(mut w) => {
            if style.box_sizing == BoxSizing::BorderBox {
                w = (w - border.horizontal() - padding.horizontal()).max(0.0);
            }
            let min_w = style.min_width.resolve(containing_block_width);
            let max_w = style
                .max_width
                .resolve(containing_block_width)
                .unwrap_or(f32::INFINITY);
            w.max(min_w).min(max_w)
        }
        None => (containing_block_width
            - border.horizontal()
            - padding.horizontal()
            - margin.horizontal())
        .max(0.0),
    };

    let is_row = style.flex_direction.is_row();
    let is_reverse = style.flex_direction.is_reverse();
    let is_wrap = style.flex_wrap != FlexWrap::Nowrap;
    let _is_wrap_reverse = style.flex_wrap == FlexWrap::WrapReverse;

    let main_size = if is_row {
        content_box_width
    } else {
        style
            .height
            .resolve(containing_block_height)
            .unwrap_or(containing_block_height)
    };
    let cross_size_available = if is_row {
        style.height.resolve(containing_block_height)
    } else {
        Some(content_box_width)
    };

    // §9.2: Collect flex items
    let children = ctx.tree.children(node);
    let mut items: Vec<FlexItem> = Vec::new();

    for &child_id in children {
        let child_style = ctx.tree.style(child_id);
        if child_style.display.is_none() {
            continue;
        }
        if child_style.position.is_absolutely_positioned() {
            // Absolutely positioned children are not flex items
            let child_frag =
                layout::layout_node(ctx, child_id, content_box_width, containing_block_height);
            fragment.children.push(child_frag);
            continue;
        }

        let item = collect_flex_item(
            ctx,
            child_id,
            is_row,
            content_box_width,
            containing_block_height,
        );
        items.push(item);
    }

    // §9.3: Determine the flex base size and hypothetical main size
    // (already done in collect_flex_item)

    // §9.4: Determine the main size of the flex container (already done above)

    // §9.5: Collect flex items into flex lines
    let lines = collect_flex_lines(&items, main_size, is_wrap);

    // §9.7: Resolve flexible lengths + §9.4/9.5: cross sizes
    let mut resolved_lines = Vec::new();

    for line in &lines {
        let resolved = resolve_flexible_lengths(line, &items, main_size);
        resolved_lines.push(resolved);
    }

    // §9.4 cross sizes: determine cross size of each item
    let mut line_cross_sizes: Vec<f32> = Vec::new();
    for (line_idx, line) in lines.iter().enumerate() {
        let mut max_cross: f32 = 0.0;
        for &item_idx in &line.item_indices {
            let item = &items[item_idx];
            let resolved_main = resolved_lines[line_idx].sizes[&item_idx];

            // Layout item with resolved main size to get cross size
            let cross = compute_item_cross_size(
                ctx,
                item,
                resolved_main,
                is_row,
                content_box_width,
                containing_block_height,
            );
            max_cross = max_cross.max(cross);
        }
        line_cross_sizes.push(max_cross);
    }

    // Determine final cross size of container
    let total_cross: f32 = line_cross_sizes.iter().sum();
    let container_cross = cross_size_available.unwrap_or(total_cross);

    // §9.9: Align items and distribute space

    // Position items
    let mut cross_offset: f32 = 0.0;

    // align-content distribution
    let extra_cross = (container_cross - total_cross).max(0.0);
    let (cross_start, cross_between) =
        distribute_alignment(style.align_content, extra_cross, resolved_lines.len());
    cross_offset += cross_start;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_cross = line_cross_sizes[line_idx];
        let resolved = &resolved_lines[line_idx];

        // Main axis alignment (justify-content)
        let total_main: f32 = line
            .item_indices
            .iter()
            .map(|&i| resolved.sizes[&i] + items[i].main_margin())
            .sum();
        let extra_main = (main_size - total_main).max(0.0);
        let (main_start, main_between) =
            distribute_justify(style.justify_content, extra_main, line.item_indices.len());

        let mut main_offset = main_start;

        let indices: Vec<usize> = if is_reverse {
            line.item_indices.iter().rev().copied().collect()
        } else {
            line.item_indices.clone()
        };

        for &item_idx in &indices {
            let item = &items[item_idx];
            let item_main = resolved.sizes[&item_idx];

            // Layout the item
            let (item_width, item_height) = if is_row {
                (item_main, line_cross)
            } else {
                (line_cross, item_main)
            };

            let mut child_frag = layout_flex_item(
                ctx,
                item.node,
                item_width,
                item_height,
                is_row,
                content_box_width,
                containing_block_height,
            );

            // Cross-axis alignment (align-items / align-self)
            let align = effective_align(style.align_items, ctx.tree.style(item.node).align_self);
            let item_cross = if is_row {
                child_frag.border_box().height
            } else {
                child_frag.border_box().width
            };
            let cross_align_offset = match align {
                AlignItems::FlexStart | AlignItems::Start => 0.0,
                AlignItems::FlexEnd | AlignItems::End => line_cross - item_cross,
                AlignItems::Center => (line_cross - item_cross) / 2.0,
                AlignItems::Stretch => 0.0,
                AlignItems::Baseline => 0.0, // simplified
            };

            // Set position
            if is_row {
                child_frag.position = Point::new(
                    main_offset + child_frag.margin.left,
                    cross_offset + cross_align_offset + child_frag.margin.top,
                );
            } else {
                child_frag.position = Point::new(
                    cross_offset + cross_align_offset + child_frag.margin.left,
                    main_offset + child_frag.margin.top,
                );
            }

            main_offset += item_main + item.main_margin() + main_between;
            fragment.children.push(child_frag);
        }

        cross_offset += line_cross + cross_between;
    }

    // Set container size
    let content_height = if is_row { container_cross } else { main_size };
    let final_height = style
        .height
        .resolve(containing_block_height)
        .unwrap_or(content_height);

    let min_h = style.min_height.resolve(containing_block_height);
    let max_h = style
        .max_height
        .resolve(containing_block_height)
        .unwrap_or(f32::INFINITY);

    fragment.size = Size::new(content_box_width, final_height.max(min_h).min(max_h));

    fragment
}

/// A collected flex item before flexible length resolution.
struct FlexItem {
    node: NodeId,
    flex_base_size: f32,
    hypothetical_main_size: f32,
    flex_grow: f32,
    flex_shrink: f32,
    min_main: f32,
    max_main: f32,
    main_margin_start: f32,
    main_margin_end: f32,
}

impl FlexItem {
    fn main_margin(&self) -> f32 {
        self.main_margin_start + self.main_margin_end
    }
}

struct FlexLine {
    item_indices: Vec<usize>,
}

struct ResolvedLine {
    sizes: std::collections::HashMap<usize, f32>,
}

fn collect_flex_item(
    ctx: &LayoutContext,
    node: NodeId,
    is_row: bool,
    containing_width: f32,
    containing_height: f32,
) -> FlexItem {
    let style = ctx.tree.style(node);
    let box_model = BoxModel::resolve(style, containing_width);

    // §9.2.3: Determine the flex base size
    let flex_base_size = match &style.flex_basis {
        LengthPercentageAuto::Length(v) => *v,
        LengthPercentageAuto::Percentage(pct) => {
            let reference = if is_row {
                containing_width
            } else {
                containing_height
            };
            reference * pct
        }
        LengthPercentageAuto::Auto => {
            // Use the main size property if definite
            let main_size = if is_row { &style.width } else { &style.height };
            let reference = if is_row {
                containing_width
            } else {
                containing_height
            };
            match main_size.resolve(reference) {
                Some(v) => v,
                None => {
                    // Content-based: layout to determine
                    let child_frag =
                        layout::layout_node(ctx, node, containing_width, containing_height);
                    if is_row {
                        child_frag.size.width
                    } else {
                        child_frag.size.height
                    }
                }
            }
        }
    };

    let (min_main, max_main) = if is_row {
        (
            style.min_width.resolve(containing_width),
            style
                .max_width
                .resolve(containing_width)
                .unwrap_or(f32::INFINITY),
        )
    } else {
        (
            style.min_height.resolve(containing_height),
            style
                .max_height
                .resolve(containing_height)
                .unwrap_or(f32::INFINITY),
        )
    };

    let hypothetical = flex_base_size.max(min_main).min(max_main);

    let (main_margin_start, main_margin_end) = if is_row {
        (box_model.margin.left, box_model.margin.right)
    } else {
        (box_model.margin.top, box_model.margin.bottom)
    };

    FlexItem {
        node,
        flex_base_size,
        hypothetical_main_size: hypothetical,
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        min_main,
        max_main,
        main_margin_start,
        main_margin_end,
    }
}

/// §9.5: Collect items into lines.
fn collect_flex_lines(items: &[FlexItem], main_size: f32, wrap: bool) -> Vec<FlexLine> {
    if items.is_empty() {
        return vec![FlexLine {
            item_indices: Vec::new(),
        }];
    }

    if !wrap {
        return vec![FlexLine {
            item_indices: (0..items.len()).collect(),
        }];
    }

    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut line_main = 0.0f32;

    for (i, item) in items.iter().enumerate() {
        let item_outer = item.hypothetical_main_size + item.main_margin();
        if !current_line.is_empty() && line_main + item_outer > main_size {
            lines.push(FlexLine {
                item_indices: std::mem::take(&mut current_line),
            });
            line_main = 0.0;
        }
        current_line.push(i);
        line_main += item_outer;
    }

    if !current_line.is_empty() {
        lines.push(FlexLine {
            item_indices: current_line,
        });
    }

    lines
}

/// §9.7: Resolve flexible lengths.
fn resolve_flexible_lengths(line: &FlexLine, items: &[FlexItem], main_size: f32) -> ResolvedLine {
    let mut sizes = std::collections::HashMap::new();

    // Calculate used space and free space
    let total_hypothetical: f32 = line
        .item_indices
        .iter()
        .map(|&i| items[i].hypothetical_main_size + items[i].main_margin())
        .sum();
    let free_space = main_size - total_hypothetical;

    let growing = free_space > 0.0;

    // §9.7 step 1: Freeze items that won't flex
    let mut frozen: Vec<bool> = vec![false; items.len()];
    let mut target_sizes: Vec<f32> = items.iter().map(|i| i.hypothetical_main_size).collect();

    for &idx in &line.item_indices {
        let item = &items[idx];
        if (growing && item.flex_grow == 0.0) || (!growing && item.flex_shrink == 0.0) {
            frozen[idx] = true;
        }
        // Freeze if hypothetical size violates constraints
        if growing && item.flex_base_size > item.hypothetical_main_size {
            frozen[idx] = true;
        }
        if !growing && item.flex_base_size < item.hypothetical_main_size {
            frozen[idx] = true;
        }
    }

    // §9.7 iterative resolution
    for _ in 0..10 {
        // Calculate remaining free space
        let frozen_space: f32 = line
            .item_indices
            .iter()
            .filter(|&&i| frozen[i])
            .map(|&i| target_sizes[i] + items[i].main_margin())
            .sum();

        let unfrozen_base: f32 = line
            .item_indices
            .iter()
            .filter(|&&i| !frozen[i])
            .map(|&i| items[i].flex_base_size + items[i].main_margin())
            .sum();

        let remaining = main_size - frozen_space - unfrozen_base;

        // Distribute space
        if growing {
            let total_grow: f32 = line
                .item_indices
                .iter()
                .filter(|&&i| !frozen[i])
                .map(|&i| items[i].flex_grow)
                .sum();

            if total_grow > 0.0 {
                for &idx in &line.item_indices {
                    if !frozen[idx] {
                        let ratio = items[idx].flex_grow / total_grow;
                        target_sizes[idx] = items[idx].flex_base_size + remaining * ratio;
                    }
                }
            }
        } else {
            let total_shrink_scaled: f32 = line
                .item_indices
                .iter()
                .filter(|&&i| !frozen[i])
                .map(|&i| items[i].flex_shrink * items[i].flex_base_size)
                .sum();

            if total_shrink_scaled > 0.0 {
                for &idx in &line.item_indices {
                    if !frozen[idx] {
                        let ratio = (items[idx].flex_shrink * items[idx].flex_base_size)
                            / total_shrink_scaled;
                        target_sizes[idx] = items[idx].flex_base_size + remaining * ratio;
                    }
                }
            }
        }

        // Clamp and freeze violated items
        let mut any_frozen = false;
        for &idx in &line.item_indices {
            if frozen[idx] {
                continue;
            }
            let clamped = target_sizes[idx]
                .max(items[idx].min_main)
                .min(items[idx].max_main);
            if (clamped - target_sizes[idx]).abs() > 0.001 {
                target_sizes[idx] = clamped;
                frozen[idx] = true;
                any_frozen = true;
            }
        }

        if !any_frozen {
            // Freeze all remaining
            for &idx in &line.item_indices {
                frozen[idx] = true;
            }
            break;
        }
    }

    // Ensure non-negative
    for &idx in &line.item_indices {
        target_sizes[idx] = target_sizes[idx].max(0.0);
        sizes.insert(idx, target_sizes[idx]);
    }

    ResolvedLine { sizes }
}

fn compute_item_cross_size(
    ctx: &LayoutContext,
    item: &FlexItem,
    main_size: f32,
    is_row: bool,
    containing_width: f32,
    containing_height: f32,
) -> f32 {
    let style = ctx.tree.style(item.node);
    let box_model = BoxModel::resolve(style, containing_width);

    if is_row {
        // Cross is height
        match style.height.resolve(containing_height) {
            Some(h) => h + box_model.border.vertical() + box_model.padding.vertical(),
            None => {
                let frag = layout::layout_node(ctx, item.node, main_size, containing_height);
                frag.border_box().height
            }
        }
    } else {
        // Cross is width
        match style.width.resolve(containing_width) {
            Some(w) => w + box_model.border.horizontal() + box_model.padding.horizontal(),
            None => {
                let frag = layout::layout_node(ctx, item.node, containing_width, main_size);
                frag.border_box().width
            }
        }
    }
}

fn layout_flex_item(
    ctx: &LayoutContext,
    node: NodeId,
    width: f32,
    height: f32,
    _is_row: bool,
    _containing_width: f32,
    _containing_height: f32,
) -> Fragment {
    // For flex items, we need to set the main size as a constraint

    layout::layout_node(ctx, node, width, height)
}

fn effective_align(container: AlignItems, item: AlignSelf) -> AlignItems {
    match item {
        AlignSelf::Auto => container,
        AlignSelf::Stretch => AlignItems::Stretch,
        AlignSelf::FlexStart => AlignItems::FlexStart,
        AlignSelf::FlexEnd => AlignItems::FlexEnd,
        AlignSelf::Center => AlignItems::Center,
        AlignSelf::Baseline => AlignItems::Baseline,
        AlignSelf::Start => AlignItems::Start,
        AlignSelf::End => AlignItems::End,
    }
}

/// Distribute space for align-content.
fn distribute_alignment(align: AlignContent, extra: f32, line_count: usize) -> (f32, f32) {
    if line_count == 0 {
        return (0.0, 0.0);
    }
    match align {
        AlignContent::FlexStart | AlignContent::Start => (0.0, 0.0),
        AlignContent::FlexEnd | AlignContent::End => (extra, 0.0),
        AlignContent::Center => (extra / 2.0, 0.0),
        AlignContent::SpaceBetween => {
            if line_count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, extra / (line_count - 1) as f32)
            }
        }
        AlignContent::SpaceAround => {
            let gap = extra / line_count as f32;
            (gap / 2.0, gap)
        }
        AlignContent::SpaceEvenly => {
            let gap = extra / (line_count + 1) as f32;
            (gap, gap)
        }
        AlignContent::Stretch => (0.0, 0.0),
    }
}

/// Distribute space for justify-content.
fn distribute_justify(justify: JustifyContent, extra: f32, item_count: usize) -> (f32, f32) {
    if item_count == 0 {
        return (0.0, 0.0);
    }
    match justify {
        JustifyContent::FlexStart | JustifyContent::Start => (0.0, 0.0),
        JustifyContent::FlexEnd | JustifyContent::End => (extra, 0.0),
        JustifyContent::Center => (extra / 2.0, 0.0),
        JustifyContent::SpaceBetween => {
            if item_count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, extra / (item_count - 1) as f32)
            }
        }
        JustifyContent::SpaceAround => {
            let gap = extra / item_count as f32;
            (gap / 2.0, gap)
        }
        JustifyContent::SpaceEvenly => {
            let gap = extra / (item_count + 1) as f32;
            (gap, gap)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{compute_layout, FixedWidthTextMeasure};
    use crate::style::ComputedStyle;
    use crate::tree::BoxTreeBuilder;

    #[test]
    fn test_flex_row_equal_items() {
        let mut builder = BoxTreeBuilder::new();
        let root_style = ComputedStyle {
            display: Display::FLEX,
            ..ComputedStyle::block()
        };
        let root = builder.root(root_style);

        for _ in 0..3 {
            let mut item_style = ComputedStyle::block();
            item_style.flex_grow = 1.0;
            item_style.height = LengthPercentageAuto::px(50.0);
            builder.element(root, item_style);
        }

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(900.0, 600.0));

        // Each item should be 300px wide (900 / 3)
        let children = tree.children(tree.root());
        for (i, &child) in children.iter().enumerate() {
            let rect = result.bounding_rect(child).unwrap();
            assert!(
                (rect.width - 300.0).abs() < 1.0,
                "item {} width: {}",
                i,
                rect.width
            );
        }
    }

    #[test]
    fn test_flex_justify_space_between() {
        let mut builder = BoxTreeBuilder::new();
        let root_style = ComputedStyle {
            display: Display::FLEX,
            justify_content: JustifyContent::SpaceBetween,
            ..ComputedStyle::block()
        };
        let root = builder.root(root_style);

        for _ in 0..3 {
            let mut item_style = ComputedStyle::block();
            item_style.width = LengthPercentageAuto::px(100.0);
            item_style.height = LengthPercentageAuto::px(50.0);
            builder.element(root, item_style);
        }

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(600.0, 600.0));

        let children = tree.children(tree.root());
        let r0 = result.bounding_rect(children[0]).unwrap();
        let r1 = result.bounding_rect(children[1]).unwrap();
        let r2 = result.bounding_rect(children[2]).unwrap();

        assert!((r0.x - 0.0).abs() < 1.0);
        assert!((r1.x - 250.0).abs() < 1.0); // (600-300)/2 + 100
        assert!((r2.x - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_distribute_justify_space_evenly() {
        let (start, between) = distribute_justify(JustifyContent::SpaceEvenly, 400.0, 3);
        assert_eq!(start, 100.0);
        assert_eq!(between, 100.0);
    }
}
