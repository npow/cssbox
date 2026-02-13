//! CSS Table Layout algorithm.
//!
//! Implements CSS 2.1 §17 — Tables.
//! Supports both `table-layout: fixed` and `table-layout: auto`.

use crate::box_model::BoxModel;
use crate::fragment::{Fragment, FragmentKind};
use crate::geometry::{Point, Size};
use crate::layout::{self, LayoutContext};
use crate::style::*;
use crate::tree::NodeId;

/// Layout a table element and its contents.
pub fn layout_table(
    ctx: &LayoutContext,
    node: NodeId,
    containing_block_width: f32,
    containing_block_height: f32,
) -> Fragment {
    let style = ctx.tree.style(node);
    let mut fragment = Fragment::new(node, FragmentKind::Box);

    // Resolve table box model
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

    // Collect table structure
    let mut table = TableStructure::new();
    collect_table_structure(ctx, node, &mut table);

    let is_fixed = style.table_layout == TableLayout::Fixed;
    let is_collapse = style.border_collapse == BorderCollapse::Collapse;
    let border_spacing = if is_collapse {
        0.0
    } else {
        style.border_spacing
    };

    // Determine column widths
    let col_widths = if is_fixed {
        fixed_table_layout(&table, content_width, border_spacing, ctx)
    } else {
        auto_table_layout(&table, content_width, border_spacing, ctx)
    };

    let num_cols = col_widths.len();
    let total_spacing = border_spacing * (num_cols + 1) as f32;
    let table_content_width = col_widths.iter().sum::<f32>() + total_spacing;

    // Layout captions (top)
    let mut cursor_y = 0.0f32;
    if style.caption_side == CaptionSide::Top {
        for &caption_node in &table.captions {
            let mut cap_frag =
                layout::layout_node(ctx, caption_node, content_width, containing_block_height);
            cap_frag.position = Point::new(cap_frag.margin.left, cursor_y);
            cursor_y += cap_frag.border_box().height + cap_frag.margin.vertical();
            fragment.children.push(cap_frag);
        }
    }

    // Layout rows
    for row in table.rows.iter() {
        let mut row_height: f32 = 0.0;
        let mut cell_fragments: Vec<(usize, Fragment)> = Vec::new();

        for cell in &row.cells {
            let col_idx = cell.col_start;
            let col_span = cell.col_span;

            // Calculate cell width
            let mut cell_width = 0.0f32;
            for c in col_idx..(col_idx + col_span).min(num_cols) {
                cell_width += col_widths[c];
                if c > col_idx {
                    cell_width += border_spacing;
                }
            }

            // Layout cell content
            let cell_frag =
                layout::layout_node(ctx, cell.node, cell_width, containing_block_height);

            let cell_total_height = cell_frag.border_box().height;
            row_height = row_height.max(cell_total_height);

            cell_fragments.push((col_idx, cell_frag));
        }

        // Resolve specified row height
        if let Some(row_node) = row.node {
            let row_style = ctx.tree.style(row_node);
            if let Some(h) = row_style.height.resolve(containing_block_height) {
                row_height = row_height.max(h);
            }
        }

        // Position cells
        for (col_idx, mut cell_frag) in cell_fragments {
            let x = col_position(&col_widths, col_idx, border_spacing);
            cell_frag.position = Point::new(x, cursor_y);

            // Vertical alignment in cell — default to top
            // TODO: implement vertical-align for table cells
            fragment.children.push(cell_frag);
        }

        cursor_y += row_height + border_spacing;
    }

    // Layout captions (bottom)
    if style.caption_side == CaptionSide::Bottom {
        for &caption_node in &table.captions {
            let mut cap_frag =
                layout::layout_node(ctx, caption_node, content_width, containing_block_height);
            cap_frag.position = Point::new(cap_frag.margin.left, cursor_y);
            cursor_y += cap_frag.border_box().height + cap_frag.margin.vertical();
            fragment.children.push(cap_frag);
        }
    }

    let final_height = style
        .height
        .resolve(containing_block_height)
        .unwrap_or(cursor_y);
    let min_h = style.min_height.resolve(containing_block_height);
    let max_h = style
        .max_height
        .resolve(containing_block_height)
        .unwrap_or(f32::INFINITY);

    fragment.size = Size::new(
        table_content_width.max(content_width),
        final_height.max(min_h).min(max_h),
    );

    fragment
}

// --- Table structure collection ---

struct TableStructure {
    rows: Vec<TableRow>,
    captions: Vec<NodeId>,
    column_specs: Vec<ColumnSpec>,
}

struct TableRow {
    node: Option<NodeId>,
    cells: Vec<TableCell>,
}

struct TableCell {
    node: NodeId,
    col_start: usize,
    col_span: usize,
}

struct ColumnSpec {
    width: Option<f32>,
}

impl TableStructure {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            captions: Vec::new(),
            column_specs: Vec::new(),
        }
    }

    fn num_columns(&self) -> usize {
        let from_cells = self
            .rows
            .iter()
            .flat_map(|r| &r.cells)
            .map(|c| c.col_start + c.col_span)
            .max()
            .unwrap_or(0);
        from_cells.max(self.column_specs.len())
    }
}

fn collect_table_structure(ctx: &LayoutContext, table_node: NodeId, table: &mut TableStructure) {
    let children = ctx.tree.children(table_node);

    for &child_id in children {
        let child_style = ctx.tree.style(child_id);

        match child_style.display.inner {
            DisplayInner::TableCaption => {
                table.captions.push(child_id);
            }
            DisplayInner::TableRow => {
                collect_row(ctx, child_id, table);
            }
            DisplayInner::TableRowGroup
            | DisplayInner::TableHeaderGroup
            | DisplayInner::TableFooterGroup => {
                // Recurse into row groups
                let group_children = ctx.tree.children(child_id);
                for &gc in group_children {
                    let gc_style = ctx.tree.style(gc);
                    if gc_style.display.inner == DisplayInner::TableRow {
                        collect_row(ctx, gc, table);
                    }
                }
            }
            DisplayInner::TableColumn | DisplayInner::TableColumnGroup => {
                // Column definitions (for width hints)
                let col_style = ctx.tree.style(child_id);
                if let Some(w) = col_style.width.resolve(0.0) {
                    table.column_specs.push(ColumnSpec { width: Some(w) });
                }
            }
            DisplayInner::TableCell => {
                // Direct cell child (implicit row)
                let row = TableRow {
                    node: None,
                    cells: vec![TableCell {
                        node: child_id,
                        col_start: 0,
                        col_span: 1,
                    }],
                };
                table.rows.push(row);
            }
            _ => {
                // Treat as anonymous table cell
            }
        }
    }
}

fn collect_row(ctx: &LayoutContext, row_node: NodeId, table: &mut TableStructure) {
    let children = ctx.tree.children(row_node);
    let mut cells = Vec::new();
    let mut col = 0;

    for &child_id in children {
        let child_style = ctx.tree.style(child_id);
        if child_style.display.is_none() {
            continue;
        }

        cells.push(TableCell {
            node: child_id,
            col_start: col,
            col_span: 1, // TODO: colspan attribute support
        });
        col += 1;
    }

    table.rows.push(TableRow {
        node: Some(row_node),
        cells,
    });
}

// --- Fixed table layout (CSS 2.1 §17.5.2.1) ---

fn fixed_table_layout(
    table: &TableStructure,
    table_width: f32,
    border_spacing: f32,
    ctx: &LayoutContext,
) -> Vec<f32> {
    let num_cols = table.num_columns().max(1);
    let total_spacing = border_spacing * (num_cols + 1) as f32;
    let available = (table_width - total_spacing).max(0.0);

    let mut widths = vec![0.0f32; num_cols];
    let mut assigned = vec![false; num_cols];

    // First: use column specs
    for (i, spec) in table.column_specs.iter().enumerate() {
        if i < num_cols {
            if let Some(w) = spec.width {
                widths[i] = w;
                assigned[i] = true;
            }
        }
    }

    // Second: use first row cell widths
    if let Some(first_row) = table.rows.first() {
        for cell in &first_row.cells {
            if cell.col_start < num_cols && !assigned[cell.col_start] {
                let cell_style = ctx.tree.style(cell.node);
                if let Some(w) = cell_style.width.resolve(available) {
                    widths[cell.col_start] = w;
                    assigned[cell.col_start] = true;
                }
            }
        }
    }

    // Third: distribute remaining space equally to unassigned columns
    let assigned_total: f32 = widths.iter().sum();
    let remaining = (available - assigned_total).max(0.0);
    let unassigned_count = assigned.iter().filter(|&&a| !a).count();

    if unassigned_count > 0 {
        let per_col = remaining / unassigned_count as f32;
        for i in 0..num_cols {
            if !assigned[i] {
                widths[i] = per_col;
            }
        }
    }

    widths
}

// --- Auto table layout (CSS 2.1 §17.5.2.2) ---

fn auto_table_layout(
    table: &TableStructure,
    table_width: f32,
    border_spacing: f32,
    ctx: &LayoutContext,
) -> Vec<f32> {
    let num_cols = table.num_columns().max(1);
    let total_spacing = border_spacing * (num_cols + 1) as f32;
    let available = (table_width - total_spacing).max(0.0);

    // Simplified auto layout: measure minimum and preferred widths
    let mut min_widths = vec![0.0f32; num_cols];
    let mut pref_widths = vec![0.0f32; num_cols];

    for row in &table.rows {
        for cell in &row.cells {
            if cell.col_span == 1 && cell.col_start < num_cols {
                let cell_style = ctx.tree.style(cell.node);

                // If cell has explicit width, use that
                if let Some(w) = cell_style.width.resolve(available) {
                    pref_widths[cell.col_start] = pref_widths[cell.col_start].max(w);
                    min_widths[cell.col_start] = min_widths[cell.col_start].max(w);
                } else {
                    // Content-based: use min/max content widths
                    let min_w = 1.0; // minimum content width placeholder
                    let pref_w = available / num_cols as f32;
                    min_widths[cell.col_start] = min_widths[cell.col_start].max(min_w);
                    pref_widths[cell.col_start] = pref_widths[cell.col_start].max(pref_w);
                }
            }
        }
    }

    // Distribute available width
    let total_pref: f32 = pref_widths.iter().sum();

    if total_pref <= available {
        // All preferred widths fit — distribute extra proportionally
        let extra = available - total_pref;
        let per_col = extra / num_cols as f32;
        let mut result = pref_widths.clone();
        for w in &mut result {
            *w += per_col;
        }
        result
    } else {
        // Need to shrink — use minimum widths as floor
        let total_min: f32 = min_widths.iter().sum();
        if total_min >= available {
            // Even minimums don't fit — use minimums
            min_widths
        } else {
            // Interpolate between min and preferred
            let flex_range = total_pref - total_min;
            let available_range = available - total_min;
            let factor = if flex_range > 0.0 {
                available_range / flex_range
            } else {
                0.0
            };

            let mut result = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                let w = min_widths[i] + (pref_widths[i] - min_widths[i]) * factor;
                result.push(w);
            }
            result
        }
    }
}

fn col_position(col_widths: &[f32], col_idx: usize, border_spacing: f32) -> f32 {
    let mut x = border_spacing;
    for i in 0..col_idx {
        x += col_widths[i] + border_spacing;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{compute_layout, FixedWidthTextMeasure};
    use crate::style::ComputedStyle;
    use crate::tree::BoxTreeBuilder;
    use crate::values::LengthPercentageAuto;

    fn make_table_style() -> ComputedStyle {
        ComputedStyle {
            display: Display::TABLE,
            ..ComputedStyle::block()
        }
    }

    fn make_row_style() -> ComputedStyle {
        ComputedStyle {
            display: Display::TABLE_ROW,
            ..ComputedStyle::block()
        }
    }

    fn make_cell_style() -> ComputedStyle {
        let mut s = ComputedStyle {
            display: Display::TABLE_CELL,
            ..ComputedStyle::block()
        };
        s.height = LengthPercentageAuto::px(30.0);
        s
    }

    #[test]
    fn test_simple_table_2x2() {
        let mut builder = BoxTreeBuilder::new();
        let root = builder.root(make_table_style());

        let row1 = builder.element(root, make_row_style());
        builder.element(row1, make_cell_style());
        builder.element(row1, make_cell_style());

        let row2 = builder.element(root, make_row_style());
        builder.element(row2, make_cell_style());
        builder.element(row2, make_cell_style());

        let tree = builder.build();
        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

        let root_layout = result.bounding_rect(tree.root()).unwrap();
        assert!(root_layout.width >= 800.0);
        assert!(root_layout.height > 0.0);
    }

    #[test]
    fn test_col_position() {
        let widths = vec![100.0, 200.0, 150.0];
        assert_eq!(col_position(&widths, 0, 5.0), 5.0);
        assert_eq!(col_position(&widths, 1, 5.0), 110.0); // 5 + 100 + 5
        assert_eq!(col_position(&widths, 2, 5.0), 315.0); // 5 + 100 + 5 + 200 + 5
    }
}
