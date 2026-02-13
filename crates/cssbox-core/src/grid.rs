//! CSS Grid Layout algorithm.
//!
//! Implements CSS Grid Layout Module Level 2.
//! Reference: https://www.w3.org/TR/css-grid-2/

use crate::box_model::BoxModel;
use crate::fragment::{Fragment, FragmentKind};
use crate::geometry::{Point, Size};
use crate::layout::{self, LayoutContext};
use crate::style::*;
use crate::tree::NodeId;

/// Layout a grid container and its items.
pub fn layout_grid(
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

    let content_width = match style.width.resolve(containing_block_width) {
        Some(mut w) => {
            if style.box_sizing == BoxSizing::BorderBox {
                w = (w - border.horizontal() - padding.horizontal()).max(0.0);
            }
            w
        }
        None => (containing_block_width
            - border.horizontal()
            - padding.horizontal()
            - margin.horizontal())
        .max(0.0),
    };

    let content_height_available = style
        .height
        .resolve(containing_block_height)
        .unwrap_or(containing_block_height);

    // §7: Define the explicit grid
    let explicit_cols = &style.grid_template_columns;
    let explicit_rows = &style.grid_template_rows;
    let col_gap = style.column_gap;
    let row_gap = style.row_gap;

    // Collect grid items
    let children = ctx.tree.children(node);
    let mut items: Vec<GridItem> = Vec::new();

    for &child_id in children {
        let child_style = ctx.tree.style(child_id);
        if child_style.display.is_none() {
            continue;
        }
        if child_style.position.is_absolutely_positioned() {
            let child_frag =
                layout::layout_node(ctx, child_id, content_width, content_height_available);
            fragment.children.push(child_frag);
            continue;
        }
        items.push(GridItem {
            node: child_id,
            row_start: resolve_grid_line(&child_style.grid_row_start),
            row_end: resolve_grid_line(&child_style.grid_row_end),
            col_start: resolve_grid_line(&child_style.grid_column_start),
            col_end: resolve_grid_line(&child_style.grid_column_end),
        });
    }

    // §8: Place items onto the grid
    let num_explicit_cols = explicit_cols.len().max(1);
    let num_explicit_rows = explicit_rows.len().max(1);

    // Auto-placement
    let mut grid_col_count = num_explicit_cols;
    let mut grid_row_count = num_explicit_rows;

    // First pass: place explicitly positioned items
    for item in &mut items {
        if item.col_start.is_none() && item.col_end.is_none() {
            continue; // auto-placed
        }
        // Resolve explicit positions
        let cs = item.col_start.unwrap_or(0);
        let ce = item.col_end.unwrap_or(cs + 1);
        let rs = item.row_start.unwrap_or(0);
        let re = item.row_end.unwrap_or(rs + 1);

        item.col_start = Some(cs);
        item.col_end = Some(ce);
        item.row_start = Some(rs);
        item.row_end = Some(re);

        grid_col_count = grid_col_count.max(ce as usize);
        grid_row_count = grid_row_count.max(re as usize);
    }

    // Second pass: auto-place remaining items
    let mut auto_cursor_row = 0i32;
    let mut auto_cursor_col = 0i32;
    let is_row_flow = matches!(
        style.grid_auto_flow,
        GridAutoFlow::Row | GridAutoFlow::RowDense
    );

    for item in &mut items {
        if item.col_start.is_some() && item.row_start.is_some() {
            continue; // already placed
        }

        if is_row_flow {
            item.col_start = Some(auto_cursor_col);
            item.col_end = Some(auto_cursor_col + 1);
            item.row_start = Some(auto_cursor_row);
            item.row_end = Some(auto_cursor_row + 1);

            auto_cursor_col += 1;
            if auto_cursor_col >= grid_col_count as i32 {
                auto_cursor_col = 0;
                auto_cursor_row += 1;
            }
        } else {
            item.row_start = Some(auto_cursor_row);
            item.row_end = Some(auto_cursor_row + 1);
            item.col_start = Some(auto_cursor_col);
            item.col_end = Some(auto_cursor_col + 1);

            auto_cursor_row += 1;
            if auto_cursor_row >= grid_row_count as i32 {
                auto_cursor_row = 0;
                auto_cursor_col += 1;
            }
        }

        grid_col_count = grid_col_count.max(item.col_end.unwrap() as usize);
        grid_row_count = grid_row_count.max(item.row_end.unwrap() as usize);
    }

    // §12: Track sizing algorithm
    let col_sizes = size_tracks(
        explicit_cols,
        &style.grid_auto_columns,
        grid_col_count,
        content_width,
        col_gap,
        &items,
        ctx,
        true,
        content_width,
        content_height_available,
    );

    let row_sizes = size_tracks(
        explicit_rows,
        &style.grid_auto_rows,
        grid_row_count,
        content_height_available,
        row_gap,
        &items,
        ctx,
        false,
        content_width,
        content_height_available,
    );

    // Compute track positions
    let col_positions = track_positions(&col_sizes, col_gap);
    let row_positions = track_positions(&row_sizes, row_gap);

    let _total_col_size = if col_sizes.is_empty() {
        0.0
    } else {
        *col_positions.last().unwrap() + *col_sizes.last().unwrap()
    };
    let total_row_size = if row_sizes.is_empty() {
        0.0
    } else {
        *row_positions.last().unwrap() + *row_sizes.last().unwrap()
    };

    // Layout and position each item
    for item in &items {
        let cs = item.col_start.unwrap() as usize;
        let ce = item.col_end.unwrap() as usize;
        let rs = item.row_start.unwrap() as usize;
        let re = item.row_end.unwrap() as usize;

        let x = if cs < col_positions.len() {
            col_positions[cs]
        } else {
            0.0
        };
        let y = if rs < row_positions.len() {
            row_positions[rs]
        } else {
            0.0
        };

        // Calculate item area size
        let mut item_width = 0.0f32;
        for c in cs..ce.min(col_sizes.len()) {
            item_width += col_sizes[c];
            if c > cs {
                item_width += col_gap;
            }
        }

        let mut item_height = 0.0f32;
        for r in rs..re.min(row_sizes.len()) {
            item_height += row_sizes[r];
            if r > rs {
                item_height += row_gap;
            }
        }

        let mut child_frag = layout::layout_node(ctx, item.node, item_width, item_height);

        // Apply alignment
        let child_style = ctx.tree.style(item.node);
        let align = effective_align_items(style.align_items, child_style.align_self);
        let justify = style.justify_content;

        let actual_w = child_frag.border_box().width;
        let actual_h = child_frag.border_box().height;

        let dx = match justify {
            JustifyContent::Center => (item_width - actual_w) / 2.0,
            JustifyContent::End | JustifyContent::FlexEnd => item_width - actual_w,
            _ => 0.0,
        };
        let dy = match align {
            AlignItems::Center => (item_height - actual_h) / 2.0,
            AlignItems::End | AlignItems::FlexEnd => item_height - actual_h,
            _ => 0.0,
        };

        child_frag.position = Point::new(
            x + dx + child_frag.margin.left,
            y + dy + child_frag.margin.top,
        );
        fragment.children.push(child_frag);
    }

    let final_height = style
        .height
        .resolve(containing_block_height)
        .unwrap_or(total_row_size);
    let min_h = style.min_height.resolve(containing_block_height);
    let max_h = style
        .max_height
        .resolve(containing_block_height)
        .unwrap_or(f32::INFINITY);

    fragment.size = Size::new(content_width, final_height.max(min_h).min(max_h));
    fragment
}

struct GridItem {
    node: NodeId,
    row_start: Option<i32>,
    row_end: Option<i32>,
    col_start: Option<i32>,
    col_end: Option<i32>,
}

fn resolve_grid_line(placement: &GridPlacement) -> Option<i32> {
    match placement {
        GridPlacement::Line(n) => Some((*n - 1).max(0)), // CSS lines are 1-based
        GridPlacement::Span(_) => None,                  // simplified
        GridPlacement::Auto => None,
        GridPlacement::Named(_) => None,
    }
}

/// §12: Track sizing algorithm (simplified).
fn size_tracks(
    explicit: &[TrackDefinition],
    auto_tracks: &[TrackSizingFunction],
    track_count: usize,
    available: f32,
    gap: f32,
    _items: &[GridItem],
    _ctx: &LayoutContext,
    _is_column: bool,
    _containing_width: f32,
    _containing_height: f32,
) -> Vec<f32> {
    let total_gaps = if track_count > 1 {
        gap * (track_count - 1) as f32
    } else {
        0.0
    };
    let available_for_tracks = available - total_gaps;

    let mut sizes = Vec::with_capacity(track_count);
    let mut total_fixed = 0.0f32;
    let mut total_fr = 0.0f32;
    let mut auto_count = 0usize;

    // Initialize track sizes from definitions
    for i in 0..track_count {
        let sizing = if i < explicit.len() {
            &explicit[i].sizing
        } else if !auto_tracks.is_empty() {
            &auto_tracks[i % auto_tracks.len()]
        } else {
            &TrackSizingFunction::Auto
        };

        match sizing {
            TrackSizingFunction::Length(px) => {
                sizes.push(*px);
                total_fixed += px;
            }
            TrackSizingFunction::Percentage(pct) => {
                let s = available_for_tracks * pct;
                sizes.push(s);
                total_fixed += s;
            }
            TrackSizingFunction::Fr(fr) => {
                sizes.push(0.0); // placeholder
                total_fr += fr;
            }
            TrackSizingFunction::Auto => {
                sizes.push(0.0); // placeholder
                auto_count += 1;
            }
            TrackSizingFunction::MinContent | TrackSizingFunction::MaxContent => {
                // Simplified: treat as auto
                sizes.push(0.0);
                auto_count += 1;
            }
            TrackSizingFunction::MinMax(min, _max) => {
                // Simplified: use min as base
                let min_val = track_fn_to_px(min, available_for_tracks);
                sizes.push(min_val);
                total_fixed += min_val;
            }
            TrackSizingFunction::FitContent(_limit) => {
                sizes.push(0.0);
                auto_count += 1;
            }
        }
    }

    // Distribute remaining space to fr tracks and auto tracks
    let remaining = (available_for_tracks - total_fixed).max(0.0);

    if total_fr > 0.0 {
        let per_fr = remaining / total_fr;
        for i in 0..track_count {
            let sizing = if i < explicit.len() {
                &explicit[i].sizing
            } else if !auto_tracks.is_empty() {
                &auto_tracks[i % auto_tracks.len()]
            } else {
                &TrackSizingFunction::Auto
            };
            if let TrackSizingFunction::Fr(fr) = sizing {
                sizes[i] = per_fr * fr;
            }
        }
        // Auto tracks get minimum
        let auto_min = 0.0;
        for i in 0..track_count {
            let sizing = if i < explicit.len() {
                &explicit[i].sizing
            } else if !auto_tracks.is_empty() {
                &auto_tracks[i % auto_tracks.len()]
            } else {
                &TrackSizingFunction::Auto
            };
            if matches!(
                sizing,
                TrackSizingFunction::Auto
                    | TrackSizingFunction::MinContent
                    | TrackSizingFunction::MaxContent
                    | TrackSizingFunction::FitContent(_)
            ) {
                sizes[i] = auto_min;
            }
        }
    } else if auto_count > 0 {
        let per_auto = remaining / auto_count as f32;
        for i in 0..track_count {
            let sizing = if i < explicit.len() {
                &explicit[i].sizing
            } else if !auto_tracks.is_empty() {
                &auto_tracks[i % auto_tracks.len()]
            } else {
                &TrackSizingFunction::Auto
            };
            if matches!(
                sizing,
                TrackSizingFunction::Auto
                    | TrackSizingFunction::MinContent
                    | TrackSizingFunction::MaxContent
                    | TrackSizingFunction::FitContent(_)
            ) {
                sizes[i] = per_auto;
            }
        }
    }

    sizes
}

fn track_fn_to_px(func: &TrackSizingFunction, available: f32) -> f32 {
    match func {
        TrackSizingFunction::Length(px) => *px,
        TrackSizingFunction::Percentage(pct) => available * pct,
        TrackSizingFunction::Auto => 0.0,
        TrackSizingFunction::MinContent => 0.0,
        TrackSizingFunction::MaxContent => available,
        TrackSizingFunction::Fr(_) => 0.0,
        TrackSizingFunction::MinMax(min, _) => track_fn_to_px(min, available),
        TrackSizingFunction::FitContent(limit) => *limit,
    }
}

fn track_positions(sizes: &[f32], gap: f32) -> Vec<f32> {
    let mut positions = Vec::with_capacity(sizes.len());
    let mut pos = 0.0f32;
    for &size in sizes.iter() {
        positions.push(pos);
        pos += size + gap;
    }
    positions
}

fn effective_align_items(container: AlignItems, item: AlignSelf) -> AlignItems {
    match item {
        AlignSelf::Auto => container,
        AlignSelf::Stretch => AlignItems::Stretch,
        AlignSelf::FlexStart | AlignSelf::Start => AlignItems::Start,
        AlignSelf::FlexEnd | AlignSelf::End => AlignItems::End,
        AlignSelf::Center => AlignItems::Center,
        AlignSelf::Baseline => AlignItems::Baseline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{compute_layout, FixedWidthTextMeasure};
    use crate::style::ComputedStyle;
    use crate::tree::BoxTreeBuilder;

    #[test]
    fn test_grid_basic_2x2() {
        let mut builder = BoxTreeBuilder::new();
        let mut root_style = ComputedStyle {
            display: Display::GRID,
            ..ComputedStyle::block()
        };
        root_style.grid_template_columns = vec![
            TrackDefinition::new(TrackSizingFunction::Fr(1.0)),
            TrackDefinition::new(TrackSizingFunction::Fr(1.0)),
        ];
        root_style.grid_template_rows = vec![
            TrackDefinition::new(TrackSizingFunction::Length(50.0)),
            TrackDefinition::new(TrackSizingFunction::Length(50.0)),
        ];
        let root = builder.root(root_style);

        // 4 items for 2x2 grid
        for _ in 0..4 {
            let child_style = ComputedStyle::block();
            builder.element(root, child_style);
        }

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let children = tree.children(tree.root());
        let r0 = result.bounding_rect(children[0]).unwrap();
        let r1 = result.bounding_rect(children[1]).unwrap();
        let r2 = result.bounding_rect(children[2]).unwrap();
        let r3 = result.bounding_rect(children[3]).unwrap();

        // Each column should be 400px wide (800 / 2)
        assert!((r0.width - 400.0).abs() < 1.0);
        assert!((r1.width - 400.0).abs() < 1.0);

        // Positions
        assert!((r0.x - 0.0).abs() < 1.0);
        assert!((r1.x - 400.0).abs() < 1.0);
        assert!((r2.x - 0.0).abs() < 1.0);
        assert!((r3.x - 400.0).abs() < 1.0);

        assert!((r0.y - 0.0).abs() < 1.0);
        assert!((r1.y - 0.0).abs() < 1.0);
        assert!((r2.y - 50.0).abs() < 1.0);
        assert!((r3.y - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_track_positions() {
        let sizes = vec![100.0, 200.0, 150.0];
        let positions = track_positions(&sizes, 10.0);
        assert_eq!(positions, vec![0.0, 110.0, 320.0]);
    }

    #[test]
    fn test_grid_with_gap() {
        let mut builder = BoxTreeBuilder::new();
        let mut root_style = ComputedStyle {
            display: Display::GRID,
            ..ComputedStyle::block()
        };
        root_style.grid_template_columns = vec![
            TrackDefinition::new(TrackSizingFunction::Fr(1.0)),
            TrackDefinition::new(TrackSizingFunction::Fr(1.0)),
        ];
        root_style.grid_template_rows =
            vec![TrackDefinition::new(TrackSizingFunction::Length(50.0))];
        root_style.column_gap = 20.0;
        let root = builder.root(root_style);

        builder.element(root, ComputedStyle::block());
        builder.element(root, ComputedStyle::block());

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(820.0, 600.0));

        let children = tree.children(tree.root());
        let r0 = result.bounding_rect(children[0]).unwrap();
        let r1 = result.bounding_rect(children[1]).unwrap();

        // (820 - 20 gap) / 2 = 400 each
        assert!((r0.width - 400.0).abs() < 1.0);
        assert!((r1.x - 420.0).abs() < 1.0); // 400 + 20 gap
    }
}
