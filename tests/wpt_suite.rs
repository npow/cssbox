//! Integration tests for the CSS layout engine.
//!
//! These tests verify the full pipeline: HTML/CSS parsing -> style resolution
//! -> layout computation -> result querying.

use cssbox_core::geometry::Size;
use cssbox_core::layout::{compute_layout, FixedWidthTextMeasure};
use cssbox_core::style::*;
use cssbox_core::tree::BoxTreeBuilder;
use cssbox_core::values::*;

// --- Block Layout Tests ---

#[test]
fn test_block_basic_stacking() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut s1 = ComputedStyle::block();
    s1.height = LengthPercentageAuto::px(100.0);
    let c1 = b.element(root, s1);

    let mut s2 = ComputedStyle::block();
    s2.height = LengthPercentageAuto::px(200.0);
    let c2 = b.element(root, s2);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let r1 = result.bounding_rect(c1).unwrap();
    let r2 = result.bounding_rect(c2).unwrap();

    assert_eq!(r1.width, 800.0);
    assert_eq!(r1.height, 100.0);
    assert_eq!(r1.y, 0.0);

    assert_eq!(r2.width, 800.0);
    assert_eq!(r2.height, 200.0);
    assert_eq!(r2.y, 100.0);

    let root_rect = result.bounding_rect(tree.root()).unwrap();
    assert_eq!(root_rect.height, 300.0);
}

#[test]
fn test_block_auto_margins_center() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut child = ComputedStyle::block();
    child.width = LengthPercentageAuto::px(400.0);
    child.height = LengthPercentageAuto::px(50.0);
    child.margin_left = LengthPercentageAuto::Auto;
    child.margin_right = LengthPercentageAuto::Auto;
    let c = b.element(root, child);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let rect = result.bounding_rect(c).unwrap();
    assert_eq!(rect.width, 400.0);
    assert_eq!(rect.x, 200.0); // centered
}

#[test]
fn test_block_border_box_sizing() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut child = ComputedStyle::block();
    child.width = LengthPercentageAuto::px(200.0);
    child.height = LengthPercentageAuto::px(100.0);
    child.box_sizing = BoxSizing::BorderBox;
    child.padding_left = LengthPercentage::px(20.0);
    child.padding_right = LengthPercentage::px(20.0);
    child.border_left_width = 5.0;
    child.border_right_width = 5.0;
    let c = b.element(root, child);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let rect = result.bounding_rect(c).unwrap();
    // border-box: total width = 200px (including padding and border)
    assert_eq!(rect.width, 200.0);
}

#[test]
fn test_block_percentage_dimensions() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut child = ComputedStyle::block();
    child.width = LengthPercentageAuto::percent(50.0);
    child.height = LengthPercentageAuto::px(80.0);
    let c = b.element(root, child);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(1000.0, 800.0));

    let rect = result.bounding_rect(c).unwrap();
    assert_eq!(rect.width, 500.0); // 50% of 1000
}

// --- Flexbox Layout Tests ---

#[test]
fn test_flex_row_distribution() {
    let mut b = BoxTreeBuilder::new();
    let mut root_style = ComputedStyle::block();
    root_style.display = Display::FLEX;
    let root = b.root(root_style);

    for _ in 0..3 {
        let mut item = ComputedStyle::block();
        item.flex_grow = 1.0;
        item.height = LengthPercentageAuto::px(50.0);
        b.element(root, item);
    }

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(900.0, 600.0));

    let children = tree.children(tree.root());
    for (i, &child) in children.iter().enumerate() {
        let rect = result.bounding_rect(child).unwrap();
        assert!(
            (rect.width - 300.0).abs() < 1.0,
            "child {} width: {}",
            i,
            rect.width
        );
    }
}

#[test]
fn test_flex_column_direction() {
    let mut b = BoxTreeBuilder::new();
    let mut root_style = ComputedStyle::block();
    root_style.display = Display::FLEX;
    root_style.flex_direction = FlexDirection::Column;
    root_style.height = LengthPercentageAuto::px(300.0);
    let root = b.root(root_style);

    for _ in 0..3 {
        let mut item = ComputedStyle::block();
        item.flex_grow = 1.0;
        b.element(root, item);
    }

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let children = tree.children(tree.root());
    for &child in children {
        let rect = result.bounding_rect(child).unwrap();
        assert!(
            (rect.height - 100.0).abs() < 1.0,
            "child height: {}",
            rect.height
        );
    }
}

// --- Grid Layout Tests ---

#[test]
fn test_grid_fr_columns() {
    let mut b = BoxTreeBuilder::new();
    let mut root_style = ComputedStyle::block();
    root_style.display = Display::GRID;
    root_style.grid_template_columns = vec![
        TrackDefinition::new(TrackSizingFunction::Fr(1.0)),
        TrackDefinition::new(TrackSizingFunction::Fr(2.0)),
    ];
    root_style.grid_template_rows = vec![TrackDefinition::new(TrackSizingFunction::Length(100.0))];
    let root = b.root(root_style);

    b.element(root, ComputedStyle::block());
    b.element(root, ComputedStyle::block());

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(900.0, 600.0));

    let children = tree.children(tree.root());
    let r0 = result.bounding_rect(children[0]).unwrap();
    let r1 = result.bounding_rect(children[1]).unwrap();

    // 1fr + 2fr = 3fr total, so 1fr = 300px, 2fr = 600px
    assert!((r0.width - 300.0).abs() < 1.0);
    assert!((r1.width - 600.0).abs() < 1.0);
}

// --- Position Tests ---

#[test]
fn test_relative_positioning() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut child = ComputedStyle::block();
    child.width = LengthPercentageAuto::px(100.0);
    child.height = LengthPercentageAuto::px(100.0);
    child.position = Position::Relative;
    child.left = LengthPercentageAuto::px(20.0);
    child.top = LengthPercentageAuto::px(10.0);
    let c = b.element(root, child);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let rect = result.bounding_rect(c).unwrap();
    assert_eq!(rect.x, 20.0);
    assert_eq!(rect.y, 10.0);
}

// --- Min/Max Constraint Tests ---

#[test]
fn test_min_width_constraint() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut child = ComputedStyle::block();
    child.width = LengthPercentageAuto::px(50.0);
    child.min_width = LengthPercentage::px(100.0);
    child.height = LengthPercentageAuto::px(50.0);
    let c = b.element(root, child);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let rect = result.bounding_rect(c).unwrap();
    assert_eq!(rect.width, 100.0); // min-width wins
}

#[test]
fn test_max_height_constraint() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut child = ComputedStyle::block();
    child.height = LengthPercentageAuto::px(500.0);
    child.max_height = LengthPercentageNone::px(200.0);
    let c = b.element(root, child);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let rect = result.bounding_rect(c).unwrap();
    assert_eq!(rect.height, 200.0); // max-height wins
}

// --- Nested Layout Tests ---

#[test]
fn test_deeply_nested_blocks() {
    let mut b = BoxTreeBuilder::new();
    let root = b.root(ComputedStyle::block());

    let mut level1 = ComputedStyle::block();
    level1.padding_left = LengthPercentage::px(10.0);
    level1.padding_right = LengthPercentage::px(10.0);
    let l1 = b.element(root, level1);

    let mut level2 = ComputedStyle::block();
    level2.padding_left = LengthPercentage::px(10.0);
    level2.padding_right = LengthPercentage::px(10.0);
    let l2 = b.element(l1, level2);

    let mut leaf = ComputedStyle::block();
    leaf.height = LengthPercentageAuto::px(50.0);
    let l3 = b.element(l2, leaf);

    let tree = b.build();
    let result = compute_layout(&tree, &FixedWidthTextMeasure, Size::new(800.0, 600.0));

    let rect = result.bounding_rect(l3).unwrap();
    // 800 - 10 - 10 (level1) - 10 - 10 (level2) = 760
    assert_eq!(rect.width, 760.0);
    assert_eq!(rect.x, 20.0); // 10 + 10
}
