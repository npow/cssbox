//! HTML parsing for the layout engine.
//!
//! Provides a simple HTML parser sufficient for WPT test files and common HTML.
//! For production use, this would be replaced with a full html5ever integration.

use std::collections::HashMap;

use crate::dom::{DomNodeId, DomTree};

/// Parse an HTML document string into a DOM tree.
pub fn parse_html(html: &str) -> DomTree {
    parse_html_simple(html)
}

/// Simple recursive HTML parser for test cases and common HTML.
pub fn parse_html_simple(html: &str) -> DomTree {
    let mut tree = DomTree::new();
    let root = tree.root();
    let html_el = tree.add_element(root, "html", HashMap::new());
    parse_children(&mut tree, html_el, html);
    tree
}

/// Simple recursive HTML parser.
fn parse_children(tree: &mut DomTree, parent: DomNodeId, html: &str) {
    let mut pos = 0;
    let bytes = html.as_bytes();
    let len = bytes.len();

    while pos < len {
        if bytes[pos] == b'<' {
            // Check for closing tag
            if pos + 1 < len && bytes[pos + 1] == b'/' {
                // Skip closing tag — return to parent parser
                return;
            }

            // Check for comment
            if html[pos..].starts_with("<!--") {
                if let Some(end) = html[pos..].find("-->") {
                    pos += end + 3;
                    continue;
                }
            }

            // Check for <!DOCTYPE
            if html[pos..].starts_with("<!") {
                while pos < len && bytes[pos] != b'>' {
                    pos += 1;
                }
                pos += 1;
                continue;
            }

            // Parse opening tag
            pos += 1; // skip '<'
            let tag_start = pos;

            // Get tag name
            while pos < len && bytes[pos] != b' ' && bytes[pos] != b'>' && bytes[pos] != b'/' {
                pos += 1;
            }
            let tag_name = html[tag_start..pos].to_string();

            // Skip <script> and <link> tags entirely
            if tag_name.eq_ignore_ascii_case("script") {
                if let Some(end) = find_tag_end(html, pos, &tag_name) {
                    pos = end;
                    continue;
                }
            }

            // Parse attributes
            let mut attrs = HashMap::new();
            while pos < len && bytes[pos] != b'>' && bytes[pos] != b'/' {
                // Skip whitespace
                while pos < len && bytes[pos] == b' ' {
                    pos += 1;
                }
                if pos >= len || bytes[pos] == b'>' || bytes[pos] == b'/' {
                    break;
                }

                // Attribute name
                let attr_start = pos;
                while pos < len
                    && bytes[pos] != b'='
                    && bytes[pos] != b' '
                    && bytes[pos] != b'>'
                    && bytes[pos] != b'/'
                {
                    pos += 1;
                }
                let attr_name = html[attr_start..pos].to_string();

                // Check for = and value
                if pos < len && bytes[pos] == b'=' {
                    pos += 1; // skip '='
                    let value = if pos < len && (bytes[pos] == b'"' || bytes[pos] == b'\'') {
                        let quote = bytes[pos];
                        pos += 1;
                        let val_start = pos;
                        while pos < len && bytes[pos] != quote {
                            pos += 1;
                        }
                        let val = html[val_start..pos].to_string();
                        if pos < len {
                            pos += 1; // skip closing quote
                        }
                        val
                    } else {
                        let val_start = pos;
                        while pos < len && bytes[pos] != b' ' && bytes[pos] != b'>' {
                            pos += 1;
                        }
                        html[val_start..pos].to_string()
                    };
                    if !attr_name.is_empty() {
                        attrs.insert(attr_name, value);
                    }
                } else if !attr_name.is_empty() {
                    attrs.insert(attr_name, String::new());
                }
            }

            // Check for self-closing
            let self_closing = pos < len && bytes[pos] == b'/';
            if self_closing {
                pos += 1;
            }
            if pos < len && bytes[pos] == b'>' {
                pos += 1;
            }

            let is_void = is_void_element(&tag_name);

            let node = tree.add_element(parent, &tag_name, attrs);

            if !self_closing && !is_void {
                // Parse children
                let _child_start = pos;
                parse_children(tree, node, &html[pos..]);
                // Skip past closing tag
                let close_tag = format!("</{}>", tag_name);
                let close_tag_lower = close_tag.to_lowercase();
                // Try case-insensitive match
                if let Some(close_pos) = html[pos..].to_lowercase().find(&close_tag_lower) {
                    pos += close_pos + close_tag.len();
                } else {
                    // No closing tag found
                    break;
                }
            }
        } else {
            // Text content
            let text_start = pos;
            while pos < len && bytes[pos] != b'<' {
                pos += 1;
            }
            let text = html[text_start..pos].trim();
            if !text.is_empty() {
                tree.add_text(parent, text);
            }
        }
    }
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag.to_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn find_tag_end(html: &str, start: usize, tag_name: &str) -> Option<usize> {
    let close_tag = format!("</{}>", tag_name);
    let lower = html[start..].to_lowercase();
    let close_lower = close_tag.to_lowercase();
    if let Some(pos) = lower.find(&close_lower) {
        Some(start + pos + close_tag.len())
    } else {
        // Self-closing or void — find the '>'
        html[start..].find('>').map(|pos| start + pos + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_html() {
        let html = r#"<div><p>Hello</p></div>"#;
        let tree = parse_html_simple(html);

        let div = tree.find_element_by_tag("div").unwrap();
        let _p = tree.find_element_by_tag("p").unwrap();
        assert!(!tree.children(div).is_empty());
    }

    #[test]
    fn test_parse_with_attributes() {
        let html = r#"<div id="test" class="box" style="width: 100px"></div>"#;
        let tree = parse_html_simple(html);
        let div = tree.find_element_by_tag("div").unwrap();
        let node = tree.node(div);
        assert_eq!(node.get_attribute("id"), Some("test"));
        assert_eq!(node.get_attribute("class"), Some("box"));
        assert_eq!(node.get_attribute("style"), Some("width: 100px"));
    }

    #[test]
    fn test_parse_nested() {
        let html = r#"<div><span>text</span><p>paragraph</p></div>"#;
        let tree = parse_html_simple(html);
        let div = tree.find_element_by_tag("div").unwrap();
        assert_eq!(tree.children(div).len(), 2);
    }

    #[test]
    fn test_parse_with_style_tag() {
        let html = r#"<style>.box { width: 100px; }</style><div class="box"></div>"#;
        let tree = parse_html_simple(html);
        let style = tree.find_element_by_tag("style").unwrap();
        let _div = tree.find_element_by_tag("div").unwrap();
        assert!(!tree.children(style).is_empty()); // text child with CSS
    }

    #[test]
    fn test_parse_void_elements() {
        let html = r#"<div><br><hr><img src="test.png"></div>"#;
        let tree = parse_html_simple(html);
        let div = tree.find_element_by_tag("div").unwrap();
        assert_eq!(tree.children(div).len(), 3);
    }
}
