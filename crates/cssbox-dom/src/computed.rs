//! Specified → computed value resolution.
//!
//! Handles CSS inheritance, initial values, and value computation.

use cssbox_core::style::ComputedStyle;
use cssbox_core::tree::{BoxTree, BoxTreeBuilder};

use crate::cascade::resolve_styles;
use crate::dom::{DomNodeId, DomTree};

/// Build a BoxTree from a DomTree with resolved styles.
///
/// This is the main integration point: takes HTML/CSS input and produces
/// the tree structure that the layout engine expects.
pub fn build_box_tree(dom: &DomTree, stylesheets: &[String]) -> BoxTree {
    // 1. Resolve styles for all elements
    let element_styles = resolve_styles(dom, stylesheets);
    let mut style_map: std::collections::HashMap<DomNodeId, ComputedStyle> =
        element_styles.into_iter().collect();

    // 2. Build box tree
    let mut builder = BoxTreeBuilder::new();

    // Find the root element (typically <html> or <body>)
    let dom_root = find_layout_root(dom);

    let root_style = style_map
        .remove(&dom_root)
        .unwrap_or_else(ComputedStyle::block);

    let box_root = builder.root(root_style);

    // 3. Recursively build children
    build_children(dom, dom_root, box_root, &mut style_map, &mut builder);

    builder.build()
}

/// Find the element to use as the layout root.
fn find_layout_root(dom: &DomTree) -> DomNodeId {
    // Prefer <body>, fall back to <html>, then document root
    if let Some(body) = dom.find_body() {
        return body;
    }
    if let Some(html) = dom.find_element_by_tag("html") {
        return html;
    }
    dom.root()
}

/// Recursively build box tree children from DOM children.
fn build_children(
    dom: &DomTree,
    dom_parent: DomNodeId,
    box_parent: cssbox_core::tree::NodeId,
    style_map: &mut std::collections::HashMap<DomNodeId, ComputedStyle>,
    builder: &mut BoxTreeBuilder,
) {
    for &child_id in dom.children(dom_parent) {
        let child_node = dom.node(child_id);

        match &child_node.kind {
            crate::dom::DomNodeKind::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    builder.text(box_parent, trimmed);
                }
            }
            crate::dom::DomNodeKind::Element { tag, .. } => {
                // Skip non-visual elements
                if matches!(
                    tag.to_lowercase().as_str(),
                    "script" | "style" | "link" | "meta" | "title" | "head"
                ) {
                    continue;
                }

                let style = style_map.remove(&child_id).unwrap_or_default();

                // Skip display: none
                if style.display.is_none() {
                    continue;
                }

                let box_child = builder.element(box_parent, style);
                build_children(dom, child_id, box_child, style_map, builder);
            }
            _ => {}
        }
    }
}

/// High-level function: parse HTML + CSS and produce a BoxTree ready for layout.
pub fn html_to_box_tree(html: &str) -> BoxTree {
    let dom = crate::html::parse_html_simple(html);

    // Extract <style> contents
    let mut stylesheets = Vec::new();
    extract_stylesheets(&dom, dom.root(), &mut stylesheets);

    build_box_tree(&dom, &stylesheets)
}

/// Extract CSS from <style> elements in the DOM.
fn extract_stylesheets(dom: &DomTree, node: DomNodeId, sheets: &mut Vec<String>) {
    let dom_node = dom.node(node);

    if let Some(tag) = dom_node.tag_name() {
        if tag.eq_ignore_ascii_case("style") {
            // Collect text content from children
            let mut css = String::new();
            for &child in dom.children(node) {
                if let Some(text) = dom.node(child).text_content() {
                    css.push_str(text);
                }
            }
            if !css.is_empty() {
                sheets.push(css);
            }
        }
    }

    for &child in dom.children(node) {
        extract_stylesheets(dom, child, sheets);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssbox_core::geometry::Size;
    use cssbox_core::layout::{compute_layout, FixedWidthTextMeasure};

    #[test]
    fn test_html_to_box_tree_basic() {
        let html = r#"
            <div style="width: 200px; height: 100px"></div>
        "#;
        let tree = html_to_box_tree(html);
        assert!(tree.len() >= 2); // root + div

        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));
        let root_rect = result.bounding_rect(tree.root()).unwrap();
        assert!(root_rect.width > 0.0);
    }

    #[test]
    fn test_html_to_box_tree_with_style_tag() {
        let html = r#"
            <style>
                .box { width: 100px; height: 50px; }
            </style>
            <div class="box"></div>
        "#;
        let tree = html_to_box_tree(html);
        assert!(tree.len() >= 2);
    }

    #[test]
    fn test_html_to_box_tree_nested() {
        let html = r#"
            <div style="width: 400px">
                <div style="width: 200px; height: 100px"></div>
                <div style="width: 200px; height: 100px"></div>
            </div>
        "#;
        let tree = html_to_box_tree(html);

        let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));
        let root_rect = result.bounding_rect(tree.root()).unwrap();
        // Root height includes the outer div which wraps two 100px children
        // The exact height depends on the DOM structure produced by parsing
        assert!(
            root_rect.height >= 200.0,
            "Root height {} should be >= 200",
            root_rect.height
        );
    }
}
