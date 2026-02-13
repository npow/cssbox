//! CSS cascade: selector matching, specificity, and style resolution.

use cssbox_core::style::ComputedStyle;

use crate::css::{apply_declarations, parse_style_attribute, parse_stylesheet, CssRule};
use crate::dom::{DomNodeId, DomTree};

/// Represents matched CSS rules with specificity for cascade ordering.
#[derive(Debug, Clone)]
struct MatchedRule {
    declarations: Vec<crate::css::CssDeclaration>,
    specificity: Specificity,
    source_order: usize,
}

/// CSS specificity (a, b, c) — id, class, type selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Specificity {
    inline: bool,
    id: u32,
    class: u32,
    type_sel: u32,
}

impl Specificity {
    fn new(id: u32, class: u32, type_sel: u32) -> Self {
        Self {
            inline: false,
            id,
            class,
            type_sel,
        }
    }
}

/// Compute the specificity of a CSS selector string.
fn compute_specificity(selector: &str) -> Specificity {
    let mut id_count = 0u32;
    let mut class_count = 0u32;
    let mut type_count = 0u32;

    // Simplified specificity calculation
    for part in selector.split(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        for segment in split_selector_segments(part) {
            if segment.starts_with('#') {
                id_count += 1;
            } else if segment.starts_with('.')
                || segment.starts_with('[')
                || segment.starts_with(':')
            {
                class_count += 1;
            } else if segment == "*" {
                // Universal selector: no specificity
            } else {
                type_count += 1;
            }
        }
    }

    Specificity::new(id_count, class_count, type_count)
}

fn split_selector_segments(selector: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let bytes = selector.as_bytes();

    for i in 1..bytes.len() {
        if bytes[i] == b'#' || bytes[i] == b'.' || bytes[i] == b'[' || bytes[i] == b':' {
            if start < i {
                segments.push(&selector[start..i]);
            }
            start = i;
        }
    }
    if start < selector.len() {
        segments.push(&selector[start..]);
    }

    segments
}

/// Check if a simple selector matches a DOM node.
fn selector_matches(tree: &DomTree, node: DomNodeId, selector: &str) -> bool {
    let _dom_node = tree.node(node);

    // Parse the selector into simple parts
    // Handle complex selectors by splitting on combinators
    let parts: Vec<&str> = selector.split_whitespace().collect();

    if parts.len() == 1 {
        return simple_selector_matches(tree, node, parts[0]);
    }

    // Descendant selector (a b)
    if parts.len() >= 2 {
        let last = parts.last().unwrap();
        if !simple_selector_matches(tree, node, last) {
            return false;
        }

        // Check ancestors for remaining parts
        let ancestor_selector = parts[..parts.len() - 1].join(" ");
        let mut current = tree.parent(node);
        while let Some(parent) = current {
            if selector_matches(tree, parent, &ancestor_selector) {
                return true;
            }
            current = tree.parent(parent);
        }
        return false;
    }

    false
}

/// Check if a single simple selector matches a node.
fn simple_selector_matches(tree: &DomTree, node: DomNodeId, selector: &str) -> bool {
    let dom_node = tree.node(node);

    if selector == "*" {
        return dom_node.is_element();
    }

    let segments = split_selector_segments(selector);

    for segment in &segments {
        if let Some(id) = segment.strip_prefix('#') {
            // ID selector
            match dom_node.get_attribute("id") {
                Some(v) if v == id => {}
                _ => return false,
            }
        } else if let Some(class) = segment.strip_prefix('.') {
            // Class selector
            match dom_node.get_attribute("class") {
                Some(classes) => {
                    if !classes.split_whitespace().any(|c| c == class) {
                        return false;
                    }
                }
                None => return false,
            }
        } else if segment.starts_with('[') {
            // Attribute selector (simplified)
            let inner = segment.trim_start_matches('[').trim_end_matches(']');
            if let Some((attr, val)) = inner.split_once('=') {
                let val = val.trim_matches('"').trim_matches('\'');
                match dom_node.get_attribute(attr) {
                    Some(v) if v == val => {}
                    _ => return false,
                }
            } else if dom_node.get_attribute(inner).is_none() {
                return false;
            }
        } else {
            // Type selector
            match dom_node.tag_name() {
                Some(tag) if tag.eq_ignore_ascii_case(segment) => {}
                _ => return false,
            }
        }
    }

    !segments.is_empty()
}

/// Resolve computed styles for all elements in a DOM tree.
pub fn resolve_styles(tree: &DomTree, stylesheets: &[String]) -> Vec<(DomNodeId, ComputedStyle)> {
    // Parse all stylesheets
    let mut rules: Vec<(CssRule, usize)> = Vec::new();
    for (i, css) in stylesheets.iter().enumerate() {
        for rule in parse_stylesheet(css) {
            rules.push((rule, i));
        }
    }

    let mut styles = Vec::new();

    for node_id in tree.iter_dfs() {
        let dom_node = tree.node(node_id);
        if !dom_node.is_element() {
            continue;
        }

        let mut style = default_style_for_tag(dom_node.tag_name().unwrap_or(""));

        // Collect matching rules
        let mut matched: Vec<MatchedRule> = Vec::new();

        for (order, (rule, _sheet_idx)) in rules.iter().enumerate() {
            // Handle comma-separated selectors
            for selector in rule.selector.split(',') {
                let selector = selector.trim();
                if selector_matches(tree, node_id, selector) {
                    matched.push(MatchedRule {
                        declarations: rule.declarations.clone(),
                        specificity: compute_specificity(selector),
                        source_order: order,
                    });
                }
            }
        }

        // Sort by specificity then source order
        matched.sort_by(|a, b| {
            a.specificity
                .cmp(&b.specificity)
                .then(a.source_order.cmp(&b.source_order))
        });

        // Apply matched rules in order
        for rule in &matched {
            apply_declarations(&mut style, &rule.declarations);
        }

        // Apply inline styles (highest specificity)
        if let Some(inline_style) = dom_node.get_attribute("style") {
            let inline_decls = parse_style_attribute(inline_style);
            apply_declarations(&mut style, &inline_decls);
        }

        styles.push((node_id, style));
    }

    styles
}

/// Default computed style based on HTML tag name.
fn default_style_for_tag(tag: &str) -> ComputedStyle {
    use cssbox_core::style::Display;
    use cssbox_core::values::LengthPercentageAuto;

    let mut style = ComputedStyle::default();

    match tag.to_lowercase().as_str() {
        "html" | "body" | "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol"
        | "li" | "article" | "section" | "nav" | "aside" | "header" | "footer" | "main"
        | "figure" | "figcaption" | "blockquote" | "pre" | "hr" | "form" | "fieldset"
        | "legend" | "details" | "summary" | "dl" | "dt" | "dd" | "address" => {
            style.display = Display::BLOCK;
        }
        "table" => {
            style.display = Display::TABLE;
        }
        "tr" => {
            style.display = Display::TABLE_ROW;
        }
        "td" | "th" => {
            style.display = Display::TABLE_CELL;
        }
        "thead" => {
            style.display = Display::TABLE_HEADER_GROUP;
        }
        "tbody" => {
            style.display = Display::TABLE_ROW_GROUP;
        }
        "tfoot" => {
            style.display = Display::TABLE_FOOTER_GROUP;
        }
        "colgroup" => {
            style.display = Display::TABLE_COLUMN_GROUP;
        }
        "col" => {
            style.display = Display::TABLE_COLUMN;
        }
        "caption" => {
            style.display = Display::TABLE_CAPTION;
        }
        _ => {
            style.display = Display::INLINE;
        }
    }

    // Default margins for some elements
    match tag.to_lowercase().as_str() {
        "body" => {
            style.margin_top = LengthPercentageAuto::px(8.0);
            style.margin_right = LengthPercentageAuto::px(8.0);
            style.margin_bottom = LengthPercentageAuto::px(8.0);
            style.margin_left = LengthPercentageAuto::px(8.0);
        }
        "p" | "blockquote" | "figure" | "ul" | "ol" | "dl" => {
            style.margin_top = LengthPercentageAuto::px(16.0);
            style.margin_bottom = LengthPercentageAuto::px(16.0);
        }
        "h1" => {
            style.margin_top = LengthPercentageAuto::px(21.44);
            style.margin_bottom = LengthPercentageAuto::px(21.44);
        }
        _ => {}
    }

    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specificity_calculation() {
        assert!(compute_specificity("#id") > compute_specificity(".class"));
        assert!(compute_specificity(".class") > compute_specificity("div"));
        assert!(compute_specificity("div.class") > compute_specificity("div"));
    }

    #[test]
    fn test_simple_selector_matching() {
        let mut tree = DomTree::new();
        let root = tree.root();
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("id".to_string(), "test".to_string());
        attrs.insert("class".to_string(), "box red".to_string());
        let div = tree.add_element(root, "div", attrs);

        assert!(simple_selector_matches(&tree, div, "div"));
        assert!(simple_selector_matches(&tree, div, "#test"));
        assert!(simple_selector_matches(&tree, div, ".box"));
        assert!(simple_selector_matches(&tree, div, ".red"));
        assert!(simple_selector_matches(&tree, div, "div.box"));
        assert!(!simple_selector_matches(&tree, div, "span"));
        assert!(!simple_selector_matches(&tree, div, ".missing"));
    }

    #[test]
    fn test_default_style_for_div() {
        let style = default_style_for_tag("div");
        assert_eq!(style.display, cssbox_core::style::Display::BLOCK);
    }
}
