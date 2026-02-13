//! CSS parsing and property resolution.
//!
//! Uses lightningcss for parsing CSS property values into typed Rust structs.

use cssbox_core::style::*;
use cssbox_core::values::*;

/// A parsed CSS declaration (property: value pair).
#[derive(Debug, Clone)]
pub struct CssDeclaration {
    pub property: String,
    pub value: String,
    pub important: bool,
}

/// Parse a CSS style string (e.g., from a `style` attribute) into declarations.
pub fn parse_style_attribute(style: &str) -> Vec<CssDeclaration> {
    let mut declarations = Vec::new();

    for decl_str in style.split(';') {
        let decl_str = decl_str.trim();
        if decl_str.is_empty() {
            continue;
        }

        if let Some((property, value)) = decl_str.split_once(':') {
            let property = property.trim().to_lowercase();
            let value = value.trim().to_string();
            let important = value.contains("!important");
            let value = value.replace("!important", "").trim().to_string();

            declarations.push(CssDeclaration {
                property,
                value,
                important,
            });
        }
    }

    declarations
}

/// Parse a CSS stylesheet string into a list of rules.
pub fn parse_stylesheet(css: &str) -> Vec<CssRule> {
    let mut rules = Vec::new();
    let mut pos = 0;
    let bytes = css.as_bytes();
    let len = bytes.len();

    while pos < len {
        // Skip whitespace and comments
        pos = skip_whitespace_comments(css, pos);
        if pos >= len {
            break;
        }

        // Find selector (everything before '{')
        let selector_start = pos;
        while pos < len && bytes[pos] != b'{' {
            pos += 1;
        }
        if pos >= len {
            break;
        }
        let selector = css[selector_start..pos].trim().to_string();
        pos += 1; // skip '{'

        // Find declarations (everything before '}')
        let decl_start = pos;
        let mut depth = 1;
        while pos < len && depth > 0 {
            if bytes[pos] == b'{' {
                depth += 1;
            } else if bytes[pos] == b'}' {
                depth -= 1;
            }
            if depth > 0 {
                pos += 1;
            }
        }
        let decl_str = &css[decl_start..pos];
        pos += 1; // skip '}'

        if !selector.is_empty() {
            let declarations = parse_style_attribute(decl_str);
            rules.push(CssRule {
                selector,
                declarations,
            });
        }
    }

    rules
}

fn skip_whitespace_comments(css: &str, mut pos: usize) -> usize {
    let bytes = css.as_bytes();
    let len = bytes.len();

    while pos < len {
        if bytes[pos].is_ascii_whitespace() {
            pos += 1;
        } else if pos + 1 < len && bytes[pos] == b'/' && bytes[pos + 1] == b'*' {
            // Block comment
            pos += 2;
            while pos + 1 < len && !(bytes[pos] == b'*' && bytes[pos + 1] == b'/') {
                pos += 1;
            }
            pos += 2;
        } else {
            break;
        }
    }
    pos
}

/// A CSS rule (selector + declarations).
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub declarations: Vec<CssDeclaration>,
}

/// Apply a set of CSS declarations to a ComputedStyle.
pub fn apply_declarations(style: &mut ComputedStyle, declarations: &[CssDeclaration]) {
    for decl in declarations {
        apply_property(style, &decl.property, &decl.value);
    }
}

/// Apply a single CSS property to a ComputedStyle.
pub fn apply_property(style: &mut ComputedStyle, property: &str, value: &str) {
    match property {
        "display" => {
            style.display = parse_display(value);
        }
        "position" => {
            style.position = match value {
                "static" => Position::Static,
                "relative" => Position::Relative,
                "absolute" => Position::Absolute,
                "fixed" => Position::Fixed,
                "sticky" => Position::Sticky,
                _ => Position::Static,
            };
        }
        "float" => {
            style.float = match value {
                "left" => Float::Left,
                "right" => Float::Right,
                "none" => Float::None,
                _ => Float::None,
            };
        }
        "clear" => {
            style.clear = match value {
                "left" => Clear::Left,
                "right" => Clear::Right,
                "both" => Clear::Both,
                "none" => Clear::None,
                _ => Clear::None,
            };
        }
        "box-sizing" => {
            style.box_sizing = match value {
                "border-box" => BoxSizing::BorderBox,
                "content-box" => BoxSizing::ContentBox,
                _ => BoxSizing::ContentBox,
            };
        }
        "width" => {
            style.width = parse_length_percentage_auto(value);
        }
        "height" => {
            style.height = parse_length_percentage_auto(value);
        }
        "min-width" => {
            style.min_width = parse_length_percentage(value);
        }
        "min-height" => {
            style.min_height = parse_length_percentage(value);
        }
        "max-width" => {
            style.max_width = parse_length_percentage_none(value);
        }
        "max-height" => {
            style.max_height = parse_length_percentage_none(value);
        }
        "margin" => {
            let edges = parse_shorthand_edges(value);
            style.margin_top = edges.0;
            style.margin_right = edges.1;
            style.margin_bottom = edges.2;
            style.margin_left = edges.3;
        }
        "margin-top" => style.margin_top = parse_length_percentage_auto(value),
        "margin-right" => style.margin_right = parse_length_percentage_auto(value),
        "margin-bottom" => style.margin_bottom = parse_length_percentage_auto(value),
        "margin-left" => style.margin_left = parse_length_percentage_auto(value),
        "padding" => {
            let edges = parse_shorthand_lp_edges(value);
            style.padding_top = edges.0;
            style.padding_right = edges.1;
            style.padding_bottom = edges.2;
            style.padding_left = edges.3;
        }
        "padding-top" => style.padding_top = parse_length_percentage(value),
        "padding-right" => style.padding_right = parse_length_percentage(value),
        "padding-bottom" => style.padding_bottom = parse_length_percentage(value),
        "padding-left" => style.padding_left = parse_length_percentage(value),
        "border-width" => {
            let w = parse_px(value);
            style.border_top_width = w;
            style.border_right_width = w;
            style.border_bottom_width = w;
            style.border_left_width = w;
        }
        "border-top-width" => style.border_top_width = parse_px(value),
        "border-right-width" => style.border_right_width = parse_px(value),
        "border-bottom-width" => style.border_bottom_width = parse_px(value),
        "border-left-width" => style.border_left_width = parse_px(value),
        "border" => {
            // Simplified: just extract width
            let parts: Vec<&str> = value.split_whitespace().collect();
            if let Some(first) = parts.first() {
                let w = parse_px(first);
                style.border_top_width = w;
                style.border_right_width = w;
                style.border_bottom_width = w;
                style.border_left_width = w;
            }
        }
        "top" => style.top = parse_length_percentage_auto(value),
        "right" => style.right = parse_length_percentage_auto(value),
        "bottom" => style.bottom = parse_length_percentage_auto(value),
        "left" => style.left = parse_length_percentage_auto(value),
        "overflow" => {
            let v = parse_overflow(value);
            style.overflow_x = v;
            style.overflow_y = v;
        }
        "overflow-x" => style.overflow_x = parse_overflow(value),
        "overflow-y" => style.overflow_y = parse_overflow(value),
        "text-align" => {
            style.text_align = match value {
                "left" => TextAlign::Left,
                "right" => TextAlign::Right,
                "center" => TextAlign::Center,
                "justify" => TextAlign::Justify,
                _ => TextAlign::Left,
            };
        }
        "line-height" => {
            style.line_height = parse_px_or_number(value, 1.2);
        }
        // Flexbox
        "flex-direction" => {
            style.flex_direction = match value {
                "row" => FlexDirection::Row,
                "row-reverse" => FlexDirection::RowReverse,
                "column" => FlexDirection::Column,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => FlexDirection::Row,
            };
        }
        "flex-wrap" => {
            style.flex_wrap = match value {
                "nowrap" => FlexWrap::Nowrap,
                "wrap" => FlexWrap::Wrap,
                "wrap-reverse" => FlexWrap::WrapReverse,
                _ => FlexWrap::Nowrap,
            };
        }
        "flex-grow" => {
            style.flex_grow = value.parse().unwrap_or(0.0);
        }
        "flex-shrink" => {
            style.flex_shrink = value.parse().unwrap_or(1.0);
        }
        "flex-basis" => {
            style.flex_basis = parse_length_percentage_auto(value);
        }
        "flex" => {
            parse_flex_shorthand(style, value);
        }
        "align-items" => {
            style.align_items = parse_align_items(value);
        }
        "align-self" => {
            style.align_self = parse_align_self(value);
        }
        "align-content" => {
            style.align_content = parse_align_content(value);
        }
        "justify-content" => {
            style.justify_content = parse_justify_content(value);
        }
        "order" => {
            style.order = value.parse().unwrap_or(0);
        }
        // Grid
        "grid-template-columns" => {
            style.grid_template_columns = parse_track_list(value);
        }
        "grid-template-rows" => {
            style.grid_template_rows = parse_track_list(value);
        }
        "row-gap" | "grid-row-gap" => {
            style.row_gap = parse_px(value);
        }
        "column-gap" | "grid-column-gap" => {
            style.column_gap = parse_px(value);
        }
        "gap" | "grid-gap" => {
            let parts: Vec<&str> = value.split_whitespace().collect();
            style.row_gap = parse_px(parts.first().unwrap_or(&"0"));
            style.column_gap = parse_px(parts.get(1).unwrap_or(parts.first().unwrap_or(&"0")));
        }
        "grid-row-start" => style.grid_row_start = parse_grid_placement(value),
        "grid-row-end" => style.grid_row_end = parse_grid_placement(value),
        "grid-column-start" => style.grid_column_start = parse_grid_placement(value),
        "grid-column-end" => style.grid_column_end = parse_grid_placement(value),
        "grid-auto-flow" => {
            style.grid_auto_flow = match value {
                "row" => GridAutoFlow::Row,
                "column" => GridAutoFlow::Column,
                "row dense" => GridAutoFlow::RowDense,
                "column dense" => GridAutoFlow::ColumnDense,
                _ => GridAutoFlow::Row,
            };
        }
        // Table
        "table-layout" => {
            style.table_layout = match value {
                "fixed" => TableLayout::Fixed,
                "auto" => TableLayout::Auto,
                _ => TableLayout::Auto,
            };
        }
        "border-collapse" => {
            style.border_collapse = match value {
                "collapse" => BorderCollapse::Collapse,
                "separate" => BorderCollapse::Separate,
                _ => BorderCollapse::Separate,
            };
        }
        "border-spacing" => {
            style.border_spacing = parse_px(value);
        }
        "caption-side" => {
            style.caption_side = match value {
                "top" => CaptionSide::Top,
                "bottom" => CaptionSide::Bottom,
                _ => CaptionSide::Top,
            };
        }
        _ => {
            // Unknown property — ignore
        }
    }
}

// --- Value parsers ---

fn parse_px(value: &str) -> f32 {
    let value = value.trim();
    if value == "0" {
        return 0.0;
    }
    if let Some(px) = value.strip_suffix("px") {
        px.trim().parse().unwrap_or(0.0)
    } else {
        value.parse().unwrap_or(0.0)
    }
}

fn parse_px_or_number(value: &str, default: f32) -> f32 {
    let value = value.trim();
    if value == "normal" {
        return default;
    }
    if let Some(px) = value.strip_suffix("px") {
        px.trim().parse().unwrap_or(default)
    } else {
        value.parse().unwrap_or(default)
    }
}

fn parse_length_percentage(value: &str) -> LengthPercentage {
    let value = value.trim();
    if value == "0" {
        return LengthPercentage::Length(0.0);
    }
    if let Some(pct) = value.strip_suffix('%') {
        LengthPercentage::Percentage(pct.trim().parse::<f32>().unwrap_or(0.0) / 100.0)
    } else if let Some(px) = value.strip_suffix("px") {
        LengthPercentage::Length(px.trim().parse().unwrap_or(0.0))
    } else {
        LengthPercentage::Length(value.parse().unwrap_or(0.0))
    }
}

fn parse_length_percentage_auto(value: &str) -> LengthPercentageAuto {
    let value = value.trim();
    if value == "auto" {
        return LengthPercentageAuto::Auto;
    }
    if value == "0" {
        return LengthPercentageAuto::Length(0.0);
    }
    if let Some(pct) = value.strip_suffix('%') {
        LengthPercentageAuto::Percentage(pct.trim().parse::<f32>().unwrap_or(0.0) / 100.0)
    } else if let Some(px) = value.strip_suffix("px") {
        LengthPercentageAuto::Length(px.trim().parse().unwrap_or(0.0))
    } else {
        LengthPercentageAuto::Length(value.parse().unwrap_or(0.0))
    }
}

fn parse_length_percentage_none(value: &str) -> LengthPercentageNone {
    let value = value.trim();
    if value == "none" {
        return LengthPercentageNone::None;
    }
    if value == "0" {
        return LengthPercentageNone::Length(0.0);
    }
    if let Some(pct) = value.strip_suffix('%') {
        LengthPercentageNone::Percentage(pct.trim().parse::<f32>().unwrap_or(0.0) / 100.0)
    } else if let Some(px) = value.strip_suffix("px") {
        LengthPercentageNone::Length(px.trim().parse().unwrap_or(0.0))
    } else {
        LengthPercentageNone::Length(value.parse().unwrap_or(0.0))
    }
}

fn parse_display(value: &str) -> Display {
    match value.trim() {
        "block" => Display::BLOCK,
        "inline" => Display::INLINE,
        "inline-block" => Display::INLINE_BLOCK,
        "flex" => Display::FLEX,
        "inline-flex" => Display::INLINE_FLEX,
        "grid" => Display::GRID,
        "inline-grid" => Display::INLINE_GRID,
        "table" => Display::TABLE,
        "table-row" => Display::TABLE_ROW,
        "table-cell" => Display::TABLE_CELL,
        "table-row-group" => Display::TABLE_ROW_GROUP,
        "table-column" => Display::TABLE_COLUMN,
        "table-column-group" => Display::TABLE_COLUMN_GROUP,
        "table-caption" => Display::TABLE_CAPTION,
        "table-header-group" => Display::TABLE_HEADER_GROUP,
        "table-footer-group" => Display::TABLE_FOOTER_GROUP,
        "flow-root" => Display::FLOW_ROOT,
        "none" => Display::NONE,
        _ => Display::INLINE,
    }
}

fn parse_overflow(value: &str) -> Overflow {
    match value.trim() {
        "visible" => Overflow::Visible,
        "hidden" => Overflow::Hidden,
        "scroll" => Overflow::Scroll,
        "auto" => Overflow::Auto,
        _ => Overflow::Visible,
    }
}

fn parse_shorthand_edges(
    value: &str,
) -> (
    LengthPercentageAuto,
    LengthPercentageAuto,
    LengthPercentageAuto,
    LengthPercentageAuto,
) {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.len() {
        1 => {
            let v = parse_length_percentage_auto(parts[0]);
            (v, v, v, v)
        }
        2 => {
            let vert = parse_length_percentage_auto(parts[0]);
            let horiz = parse_length_percentage_auto(parts[1]);
            (vert, horiz, vert, horiz)
        }
        3 => {
            let top = parse_length_percentage_auto(parts[0]);
            let horiz = parse_length_percentage_auto(parts[1]);
            let bottom = parse_length_percentage_auto(parts[2]);
            (top, horiz, bottom, horiz)
        }
        4 => (
            parse_length_percentage_auto(parts[0]),
            parse_length_percentage_auto(parts[1]),
            parse_length_percentage_auto(parts[2]),
            parse_length_percentage_auto(parts[3]),
        ),
        _ => (
            LengthPercentageAuto::px(0.0),
            LengthPercentageAuto::px(0.0),
            LengthPercentageAuto::px(0.0),
            LengthPercentageAuto::px(0.0),
        ),
    }
}

fn parse_shorthand_lp_edges(
    value: &str,
) -> (
    LengthPercentage,
    LengthPercentage,
    LengthPercentage,
    LengthPercentage,
) {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.len() {
        1 => {
            let v = parse_length_percentage(parts[0]);
            (v, v, v, v)
        }
        2 => {
            let vert = parse_length_percentage(parts[0]);
            let horiz = parse_length_percentage(parts[1]);
            (vert, horiz, vert, horiz)
        }
        3 => {
            let top = parse_length_percentage(parts[0]);
            let horiz = parse_length_percentage(parts[1]);
            let bottom = parse_length_percentage(parts[2]);
            (top, horiz, bottom, horiz)
        }
        4 => (
            parse_length_percentage(parts[0]),
            parse_length_percentage(parts[1]),
            parse_length_percentage(parts[2]),
            parse_length_percentage(parts[3]),
        ),
        _ => (
            LengthPercentage::Length(0.0),
            LengthPercentage::Length(0.0),
            LengthPercentage::Length(0.0),
            LengthPercentage::Length(0.0),
        ),
    }
}

fn parse_flex_shorthand(style: &mut ComputedStyle, value: &str) {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.len() {
        1 => {
            if parts[0] == "none" {
                style.flex_grow = 0.0;
                style.flex_shrink = 0.0;
                style.flex_basis = LengthPercentageAuto::Auto;
            } else if parts[0] == "auto" {
                style.flex_grow = 1.0;
                style.flex_shrink = 1.0;
                style.flex_basis = LengthPercentageAuto::Auto;
            } else if let Ok(grow) = parts[0].parse::<f32>() {
                style.flex_grow = grow;
                style.flex_shrink = 1.0;
                style.flex_basis = LengthPercentageAuto::px(0.0);
            }
        }
        2 => {
            style.flex_grow = parts[0].parse().unwrap_or(0.0);
            if let Ok(shrink) = parts[1].parse::<f32>() {
                style.flex_shrink = shrink;
                style.flex_basis = LengthPercentageAuto::px(0.0);
            } else {
                style.flex_shrink = 1.0;
                style.flex_basis = parse_length_percentage_auto(parts[1]);
            }
        }
        3 => {
            style.flex_grow = parts[0].parse().unwrap_or(0.0);
            style.flex_shrink = parts[1].parse().unwrap_or(1.0);
            style.flex_basis = parse_length_percentage_auto(parts[2]);
        }
        _ => {}
    }
}

fn parse_align_items(value: &str) -> AlignItems {
    match value.trim() {
        "stretch" => AlignItems::Stretch,
        "flex-start" | "start" => AlignItems::FlexStart,
        "flex-end" | "end" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "baseline" => AlignItems::Baseline,
        _ => AlignItems::Stretch,
    }
}

fn parse_align_self(value: &str) -> AlignSelf {
    match value.trim() {
        "auto" => AlignSelf::Auto,
        "stretch" => AlignSelf::Stretch,
        "flex-start" | "start" => AlignSelf::FlexStart,
        "flex-end" | "end" => AlignSelf::FlexEnd,
        "center" => AlignSelf::Center,
        "baseline" => AlignSelf::Baseline,
        _ => AlignSelf::Auto,
    }
}

fn parse_align_content(value: &str) -> AlignContent {
    match value.trim() {
        "stretch" => AlignContent::Stretch,
        "flex-start" | "start" => AlignContent::FlexStart,
        "flex-end" | "end" => AlignContent::FlexEnd,
        "center" => AlignContent::Center,
        "space-between" => AlignContent::SpaceBetween,
        "space-around" => AlignContent::SpaceAround,
        "space-evenly" => AlignContent::SpaceEvenly,
        _ => AlignContent::Stretch,
    }
}

fn parse_justify_content(value: &str) -> JustifyContent {
    match value.trim() {
        "flex-start" | "start" => JustifyContent::FlexStart,
        "flex-end" | "end" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "space-between" => JustifyContent::SpaceBetween,
        "space-around" => JustifyContent::SpaceAround,
        "space-evenly" => JustifyContent::SpaceEvenly,
        _ => JustifyContent::FlexStart,
    }
}

fn parse_track_list(value: &str) -> Vec<TrackDefinition> {
    let mut tracks = Vec::new();

    // Simple tokenizer for track values
    for token in split_track_tokens(value) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        tracks.push(TrackDefinition::new(parse_track_sizing(token)));
    }

    tracks
}

fn split_track_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0;

    for ch in value.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current.push(ch);
            }
            ' ' if paren_depth == 0 => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn parse_track_sizing(token: &str) -> TrackSizingFunction {
    if let Some(fr) = token.strip_suffix("fr") {
        TrackSizingFunction::Fr(fr.parse().unwrap_or(1.0))
    } else if let Some(pct) = token.strip_suffix('%') {
        TrackSizingFunction::Percentage(pct.parse::<f32>().unwrap_or(0.0) / 100.0)
    } else if let Some(px) = token.strip_suffix("px") {
        TrackSizingFunction::Length(px.parse().unwrap_or(0.0))
    } else if token == "auto" {
        TrackSizingFunction::Auto
    } else if token == "min-content" {
        TrackSizingFunction::MinContent
    } else if token == "max-content" {
        TrackSizingFunction::MaxContent
    } else if token.starts_with("minmax(") {
        parse_minmax(token)
    } else if token.starts_with("fit-content(") {
        let inner = &token["fit-content(".len()..token.len() - 1];
        TrackSizingFunction::FitContent(parse_px(inner))
    } else {
        TrackSizingFunction::Length(token.parse().unwrap_or(0.0))
    }
}

fn parse_minmax(token: &str) -> TrackSizingFunction {
    let inner = &token["minmax(".len()..token.len() - 1];
    if let Some((min, max)) = inner.split_once(',') {
        TrackSizingFunction::MinMax(
            Box::new(parse_track_sizing(min.trim())),
            Box::new(parse_track_sizing(max.trim())),
        )
    } else {
        TrackSizingFunction::Auto
    }
}

fn parse_grid_placement(value: &str) -> GridPlacement {
    let value = value.trim();
    if value == "auto" {
        return GridPlacement::Auto;
    }
    if let Some(span_val) = value.strip_prefix("span ") {
        if let Ok(n) = span_val.trim().parse::<u32>() {
            return GridPlacement::Span(n);
        }
    }
    if let Ok(n) = value.parse::<i32>() {
        return GridPlacement::Line(n);
    }
    GridPlacement::Named(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_style_attribute() {
        let decls = parse_style_attribute("width: 100px; height: 50px; margin: 10px auto");
        assert_eq!(decls.len(), 3);
        assert_eq!(decls[0].property, "width");
        assert_eq!(decls[0].value, "100px");
    }

    #[test]
    fn test_apply_display() {
        let mut style = ComputedStyle::default();
        apply_property(&mut style, "display", "flex");
        assert_eq!(style.display, Display::FLEX);
    }

    #[test]
    fn test_apply_margin_shorthand() {
        let mut style = ComputedStyle::default();
        apply_property(&mut style, "margin", "10px 20px");
        assert_eq!(style.margin_top, LengthPercentageAuto::px(10.0));
        assert_eq!(style.margin_right, LengthPercentageAuto::px(20.0));
        assert_eq!(style.margin_bottom, LengthPercentageAuto::px(10.0));
        assert_eq!(style.margin_left, LengthPercentageAuto::px(20.0));
    }

    #[test]
    fn test_parse_percentage() {
        let lpa = parse_length_percentage_auto("50%");
        assert_eq!(lpa, LengthPercentageAuto::Percentage(0.5));
    }

    #[test]
    fn test_parse_stylesheet() {
        let css = "div { width: 100px; height: 50px; } .box { display: flex; }";
        let rules = parse_stylesheet(css);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selector, "div");
        assert_eq!(rules[0].declarations.len(), 2);
        assert_eq!(rules[1].selector, ".box");
    }

    #[test]
    fn test_parse_track_list() {
        let tracks = parse_track_list("1fr 200px auto");
        assert_eq!(tracks.len(), 3);
        assert!(matches!(tracks[0].sizing, TrackSizingFunction::Fr(f) if (f - 1.0).abs() < 0.001));
        assert!(
            matches!(tracks[1].sizing, TrackSizingFunction::Length(v) if (v - 200.0).abs() < 0.001)
        );
        assert!(matches!(tracks[2].sizing, TrackSizingFunction::Auto));
    }
}
