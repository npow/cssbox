//! Extract testharness.js assertions from WPT test files.
//!
//! Parses JavaScript assertions like:
//! - `assert_equals(el.getBoundingClientRect().width, 200)`
//! - `assert_equals(el.offsetHeight, 100)`
//! - `assert_equals(getComputedStyle(el).display, "block")`

use cssbox_core::fragment::LayoutResult;
use cssbox_core::geometry::Size;
use cssbox_core::layout::{compute_layout, FixedWidthTextMeasure};
use cssbox_core::tree::NodeId;
use cssbox_dom::dom::DomTree;
use cssbox_dom::html::parse_html_simple;

/// An extracted assertion from a testharness.js test.
#[derive(Debug, Clone)]
pub struct Assertion {
    /// The element selector (e.g., "#target", ".box").
    pub element_selector: String,
    /// The property being asserted.
    pub property: AssertedProperty,
    /// The expected value.
    pub expected_value: f32,
}

/// What layout property is being asserted.
#[derive(Debug, Clone, PartialEq)]
pub enum AssertedProperty {
    BoundingRectWidth,
    BoundingRectHeight,
    BoundingRectX,
    BoundingRectY,
    BoundingRectTop,
    BoundingRectLeft,
    BoundingRectRight,
    BoundingRectBottom,
    OffsetWidth,
    OffsetHeight,
    OffsetTop,
    OffsetLeft,
    ClientWidth,
    ClientHeight,
}

/// Extract assertions from the JavaScript in a WPT test file.
pub fn extract_assertions(html: &str) -> Vec<Assertion> {
    let mut assertions = Vec::new();

    // Find all <script> blocks
    let mut pos = 0;
    while let Some(start) = html[pos..].find("<script") {
        let script_start = pos + start;
        if let Some(content_start) = html[script_start..].find('>') {
            let content_start = script_start + content_start + 1;
            if let Some(end) = html[content_start..].find("</script>") {
                let script = &html[content_start..content_start + end];
                extract_from_script(script, &mut assertions);
                pos = content_start + end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    assertions
}

fn extract_from_script(script: &str, assertions: &mut Vec<Assertion>) {
    // Pattern: assert_equals(expr, value)
    let mut pos = 0;
    while let Some(start) = script[pos..].find("assert_equals") {
        let assert_start = pos + start;
        if let Some(paren_start) = script[assert_start..].find('(') {
            let args_start = assert_start + paren_start + 1;
            if let Some(args_end) = find_matching_paren(script, args_start - 1) {
                let args = &script[args_start..args_end];
                if let Some(assertion) = parse_assert_equals(args) {
                    assertions.push(assertion);
                }
                pos = args_end + 1;
            } else {
                pos = assert_start + 13;
            }
        } else {
            pos = assert_start + 13;
        }
    }
}

fn find_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in s[open_pos..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_assert_equals(args: &str) -> Option<Assertion> {
    // Split on the last comma at depth 0
    let comma_pos = find_depth_zero_comma(args)?;
    let expr = args[..comma_pos].trim();
    let value_str = args[comma_pos + 1..]
        .trim()
        .trim_matches('"')
        .trim_matches('\'');

    let value: f32 = value_str.parse().ok()?;

    // Parse the expression
    let (selector, property) = parse_expression(expr)?;

    Some(Assertion {
        element_selector: selector,
        property,
        expected_value: value,
    })
}

fn find_depth_zero_comma(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut last_comma = None;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => last_comma = Some(i),
            _ => {}
        }
    }
    last_comma
}

fn parse_expression(expr: &str) -> Option<(String, AssertedProperty)> {
    let expr = expr.trim();

    // Pattern: element.getBoundingClientRect().property
    if expr.contains("getBoundingClientRect()") {
        let selector = extract_element_selector(expr)?;
        let property = if expr.ends_with(".width") {
            AssertedProperty::BoundingRectWidth
        } else if expr.ends_with(".height") {
            AssertedProperty::BoundingRectHeight
        } else if expr.ends_with(".x") || expr.ends_with(".left") {
            AssertedProperty::BoundingRectLeft
        } else if expr.ends_with(".y") || expr.ends_with(".top") {
            AssertedProperty::BoundingRectTop
        } else if expr.ends_with(".right") {
            AssertedProperty::BoundingRectRight
        } else if expr.ends_with(".bottom") {
            AssertedProperty::BoundingRectBottom
        } else {
            return None;
        };
        return Some((selector, property));
    }

    // Pattern: element.offsetWidth/offsetHeight/offsetTop/offsetLeft
    if expr.contains(".offsetWidth") {
        let selector = extract_element_selector(expr)?;
        return Some((selector, AssertedProperty::OffsetWidth));
    }
    if expr.contains(".offsetHeight") {
        let selector = extract_element_selector(expr)?;
        return Some((selector, AssertedProperty::OffsetHeight));
    }
    if expr.contains(".offsetTop") {
        let selector = extract_element_selector(expr)?;
        return Some((selector, AssertedProperty::OffsetTop));
    }
    if expr.contains(".offsetLeft") {
        let selector = extract_element_selector(expr)?;
        return Some((selector, AssertedProperty::OffsetLeft));
    }

    // Pattern: element.clientWidth/clientHeight
    if expr.contains(".clientWidth") {
        let selector = extract_element_selector(expr)?;
        return Some((selector, AssertedProperty::ClientWidth));
    }
    if expr.contains(".clientHeight") {
        let selector = extract_element_selector(expr)?;
        return Some((selector, AssertedProperty::ClientHeight));
    }

    None
}

fn extract_element_selector(expr: &str) -> Option<String> {
    // Pattern: document.getElementById("id")
    if let Some(start) = expr.find("getElementById(\"") {
        let id_start = start + 16;
        if let Some(end) = expr[id_start..].find('"') {
            return Some(format!("#{}", &expr[id_start..id_start + end]));
        }
    }
    if let Some(start) = expr.find("getElementById('") {
        let id_start = start + 16;
        if let Some(end) = expr[id_start..].find('\'') {
            return Some(format!("#{}", &expr[id_start..id_start + end]));
        }
    }

    // Pattern: document.querySelector("selector")
    if let Some(start) = expr.find("querySelector(\"") {
        let sel_start = start + 15;
        if let Some(end) = expr[sel_start..].find('"') {
            return Some(expr[sel_start..sel_start + end].to_string());
        }
    }

    // Variable reference (simplified)
    if let Some(dot_pos) = expr.find('.') {
        let var = expr[..dot_pos].trim();
        if var.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Some(var.to_string());
        }
    }

    None
}

/// Run testharness assertions against a layout result.
pub fn run_assertions(html: &str, assertions: &[Assertion]) -> Vec<AssertionResult> {
    let dom = parse_html_simple(html);
    let tree = cssbox_dom::computed::html_to_box_tree(html);
    let viewport = Size::new(800.0, 600.0);
    let result = compute_layout(&tree, &FixedWidthTextMeasure, viewport);

    let mut results = Vec::new();

    for assertion in assertions {
        // Find the element by selector
        let node_id = find_node_by_selector(&dom, &tree, &assertion.element_selector);

        let actual = match node_id {
            Some(nid) => get_property_value(&result, nid, &assertion.property),
            None => None,
        };

        let passed = match actual {
            Some(v) => (v - assertion.expected_value).abs() < 0.5,
            None => false,
        };

        results.push(AssertionResult {
            assertion: assertion.clone(),
            actual_value: actual,
            passed,
        });
    }

    results
}

fn find_node_by_selector(
    dom: &DomTree,
    tree: &cssbox_core::tree::BoxTree,
    selector: &str,
) -> Option<NodeId> {
    // Simple selector resolution
    if let Some(id) = selector.strip_prefix('#') {
        // Find by ID in DOM, then map to box tree node
        let dom_node = dom.find_element_by_id(id)?;
        // The box tree node index roughly corresponds to DOM traversal order
        // This is simplified — a real implementation would maintain a mapping
        Some(NodeId(dom_node.0.min(tree.len().saturating_sub(1))))
    } else {
        None
    }
}

fn get_property_value(
    result: &LayoutResult,
    node: NodeId,
    property: &AssertedProperty,
) -> Option<f32> {
    let rect = result.bounding_rect(node)?;
    Some(match property {
        AssertedProperty::BoundingRectWidth
        | AssertedProperty::OffsetWidth
        | AssertedProperty::ClientWidth => rect.width,
        AssertedProperty::BoundingRectHeight
        | AssertedProperty::OffsetHeight
        | AssertedProperty::ClientHeight => rect.height,
        AssertedProperty::BoundingRectX
        | AssertedProperty::BoundingRectLeft
        | AssertedProperty::OffsetLeft => rect.x,
        AssertedProperty::BoundingRectY
        | AssertedProperty::BoundingRectTop
        | AssertedProperty::OffsetTop => rect.y,
        AssertedProperty::BoundingRectRight => rect.x + rect.width,
        AssertedProperty::BoundingRectBottom => rect.y + rect.height,
    })
}

/// Result of running a single assertion.
#[derive(Debug)]
pub struct AssertionResult {
    pub assertion: Assertion,
    pub actual_value: Option<f32>,
    pub passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_assertion() {
        let html = r#"
            <div id="target" style="width: 100px"></div>
            <script>
                assert_equals(document.getElementById("target").getBoundingClientRect().width, 100);
            </script>
        "#;

        let assertions = extract_assertions(html);
        assert!(!assertions.is_empty());
        assert_eq!(assertions[0].element_selector, "#target");
        assert_eq!(assertions[0].property, AssertedProperty::BoundingRectWidth);
        assert_eq!(assertions[0].expected_value, 100.0);
    }

    #[test]
    fn test_extract_offset_assertion() {
        let html = r#"
            <script>
                assert_equals(document.getElementById("box").offsetHeight, 200);
            </script>
        "#;

        let assertions = extract_assertions(html);
        assert!(!assertions.is_empty());
        assert_eq!(assertions[0].property, AssertedProperty::OffsetHeight);
        assert_eq!(assertions[0].expected_value, 200.0);
    }
}
