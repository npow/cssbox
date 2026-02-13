//! Reftest comparison: compare layout trees of two HTML documents.

use cssbox_core::fragment::Fragment;
use cssbox_core::geometry::Size;
use cssbox_core::layout::{compute_layout, FixedWidthTextMeasure};
use cssbox_dom::computed::html_to_box_tree;

/// Tolerance for floating-point comparison (0.5px).
const TOLERANCE: f32 = 0.5;

/// Result of a reftest comparison.
#[derive(Debug)]
pub struct ReftestResult {
    pub passed: bool,
    pub differences: Vec<ReftestDifference>,
}

/// A specific difference found between test and reference layouts.
#[derive(Debug)]
pub struct ReftestDifference {
    pub description: String,
    pub test_value: String,
    pub reference_value: String,
}

/// Compare the layout of two HTML documents.
///
/// Returns true if the layout trees are structurally equivalent
/// within tolerance (same number of boxes, same positions/sizes).
pub fn compare_layouts(test_html: &str, reference_html: &str) -> ReftestResult {
    let test_tree = html_to_box_tree(test_html);
    let ref_tree = html_to_box_tree(reference_html);
    let viewport = Size::new(800.0, 600.0);

    let test_result = compute_layout(&test_tree, &FixedWidthTextMeasure, viewport);
    let ref_result = compute_layout(&ref_tree, &FixedWidthTextMeasure, viewport);

    let mut differences = Vec::new();
    compare_fragments(&test_result.root, &ref_result.root, "", &mut differences);

    ReftestResult {
        passed: differences.is_empty(),
        differences,
    }
}

/// Recursively compare two fragment trees.
fn compare_fragments(
    test: &Fragment,
    reference: &Fragment,
    path: &str,
    differences: &mut Vec<ReftestDifference>,
) {
    // Compare positions
    if !approx_eq(test.position.x, reference.position.x) {
        differences.push(ReftestDifference {
            description: format!("{} position.x", path),
            test_value: format!("{:.1}", test.position.x),
            reference_value: format!("{:.1}", reference.position.x),
        });
    }
    if !approx_eq(test.position.y, reference.position.y) {
        differences.push(ReftestDifference {
            description: format!("{} position.y", path),
            test_value: format!("{:.1}", test.position.y),
            reference_value: format!("{:.1}", reference.position.y),
        });
    }

    // Compare sizes
    if !approx_eq(test.size.width, reference.size.width) {
        differences.push(ReftestDifference {
            description: format!("{} size.width", path),
            test_value: format!("{:.1}", test.size.width),
            reference_value: format!("{:.1}", reference.size.width),
        });
    }
    if !approx_eq(test.size.height, reference.size.height) {
        differences.push(ReftestDifference {
            description: format!("{} size.height", path),
            test_value: format!("{:.1}", test.size.height),
            reference_value: format!("{:.1}", reference.size.height),
        });
    }

    // Compare children (structural match)
    let min_children = test.children.len().min(reference.children.len());
    for i in 0..min_children {
        let child_path = format!("{}/child[{}]", path, i);
        compare_fragments(
            &test.children[i],
            &reference.children[i],
            &child_path,
            differences,
        );
    }

    // Note structural differences
    if test.children.len() != reference.children.len() {
        differences.push(ReftestDifference {
            description: format!("{} child count", path),
            test_value: format!("{}", test.children.len()),
            reference_value: format!("{}", reference.children.len()),
        });
    }
}

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_layouts_pass() {
        let html = r#"<div style="width: 100px; height: 100px"></div>"#;
        let result = compare_layouts(html, html);
        assert!(
            result.passed,
            "Identical HTML should pass: {:?}",
            result.differences
        );
    }

    #[test]
    fn test_different_sizes_fail() {
        let test_html = r#"<div style="width: 100px; height: 100px"></div>"#;
        let ref_html = r#"<div style="width: 200px; height: 100px"></div>"#;
        let result = compare_layouts(test_html, ref_html);
        assert!(!result.passed, "Different sizes should fail");
    }
}
