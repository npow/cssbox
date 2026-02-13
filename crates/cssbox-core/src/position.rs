//! CSS positioning: relative, absolute, fixed, sticky.
//!
//! Implements CSS 2.1 §9.3 and css-position-3.

use crate::box_model::BoxModel;
use crate::fragment::Fragment;
use crate::geometry::{Point, Size};
use crate::style::{ComputedStyle, Position};
use crate::tree::BoxTree;

/// Resolve positioned elements after initial layout.
///
/// This walks the fragment tree and adjusts positions for:
/// - `position: relative` — offset from normal flow position
/// - `position: absolute` — positioned relative to containing block
/// - `position: fixed` — positioned relative to viewport
pub fn resolve_positioned(tree: &BoxTree, mut root: Fragment, viewport: Size) -> Fragment {
    resolve_fragment(tree, &mut root, viewport, viewport);
    root
}

fn resolve_fragment(
    tree: &BoxTree,
    fragment: &mut Fragment,
    containing_block_size: Size,
    viewport: Size,
) {
    let style = tree.style(fragment.node);

    match style.position {
        Position::Relative => {
            apply_relative_offset(fragment, style, containing_block_size);
        }
        Position::Absolute => {
            resolve_absolute_position(fragment, style, containing_block_size);
        }
        Position::Fixed => {
            resolve_absolute_position(fragment, style, viewport);
        }
        Position::Sticky => {
            // Sticky is treated like relative for initial layout
            apply_relative_offset(fragment, style, containing_block_size);
        }
        Position::Static => {}
    }

    // Determine containing block for positioned descendants
    let child_cb = if style.position.is_positioned() || style.position == Position::Static {
        // If this element is positioned, it becomes the containing block for
        // absolutely positioned descendants
        Size::new(
            fragment.size.width + fragment.padding.horizontal() + fragment.border.horizontal(),
            fragment.size.height + fragment.padding.vertical() + fragment.border.vertical(),
        )
    } else {
        containing_block_size
    };

    // Recurse into children
    for child in &mut fragment.children {
        resolve_fragment(tree, child, child_cb, viewport);
    }
}

/// Apply relative positioning offsets.
/// CSS 2.1 §9.4.3: Relative positioning offsets the box from its normal flow position.
fn apply_relative_offset(fragment: &mut Fragment, style: &ComputedStyle, cb_size: Size) {
    let dx = resolve_offset_pair(&style.left, &style.right, cb_size.width);
    let dy = resolve_offset_pair(&style.top, &style.bottom, cb_size.height);

    fragment.position.x += dx;
    fragment.position.y += dy;
}

/// Resolve absolute positioning.
/// CSS 2.1 §10.3.7 and §10.6.4: Absolutely positioned, non-replaced elements.
fn resolve_absolute_position(fragment: &mut Fragment, style: &ComputedStyle, cb_size: Size) {
    let border = BoxModel::resolve_border(style);
    let padding = BoxModel::resolve_padding(style, cb_size.width);

    // Resolve horizontal position and width
    let (x, width) = resolve_absolute_axis(
        &style.left,
        &style.right,
        &style.width,
        &style.margin_left,
        &style.margin_right,
        border.left + padding.left,
        border.right + padding.right,
        cb_size.width,
        fragment.size.width,
    );

    // Resolve vertical position and height
    let (y, height) = resolve_absolute_axis(
        &style.top,
        &style.bottom,
        &style.height,
        &style.margin_top,
        &style.margin_bottom,
        border.top + padding.top,
        border.bottom + padding.bottom,
        cb_size.height,
        fragment.size.height,
    );

    fragment.position = Point::new(x, y);
    if width >= 0.0 {
        fragment.size.width = width;
    }
    if height >= 0.0 {
        fragment.size.height = height;
    }
    fragment.border = border;
    fragment.padding = padding;
}

/// Resolve one axis of absolute positioning.
///
/// The constraint equation:
/// start + margin_start + border_start + padding_start + width +
/// padding_end + border_end + margin_end + end = containing_block_size
///
/// Returns (position, content_size). Content_size is -1 if unchanged.
fn resolve_absolute_axis(
    start: &crate::values::LengthPercentageAuto,
    end: &crate::values::LengthPercentageAuto,
    size: &crate::values::LengthPercentageAuto,
    margin_start: &crate::values::LengthPercentageAuto,
    margin_end: &crate::values::LengthPercentageAuto,
    border_padding_start: f32,
    border_padding_end: f32,
    cb_size: f32,
    intrinsic_size: f32,
) -> (f32, f32) {
    let start_val = start.resolve(cb_size);
    let end_val = end.resolve(cb_size);
    let size_val = size.resolve(cb_size);
    let ms = margin_start.resolve(cb_size).unwrap_or(0.0);
    let me = margin_end.resolve(cb_size).unwrap_or(0.0);

    match (start_val, end_val, size_val) {
        // All three specified: over-constrained, ignore end
        (Some(s), Some(_e), Some(w)) => {
            let pos = s + ms;
            (pos, w)
        }
        // Start and size specified
        (Some(s), _, Some(w)) => {
            let pos = s + ms;
            (pos, w)
        }
        // End and size specified
        (_, Some(e), Some(w)) => {
            let pos = cb_size - e - me - w - border_padding_start - border_padding_end;
            (pos, w)
        }
        // Start and end specified (stretch)
        (Some(s), Some(e), None) => {
            let pos = s + ms;
            let w = cb_size - s - e - ms - me - border_padding_start - border_padding_end;
            (pos, w.max(0.0))
        }
        // Only start specified
        (Some(s), None, None) => {
            let pos = s + ms;
            (pos, intrinsic_size)
        }
        // Only end specified
        (None, Some(e), None) => {
            let pos = cb_size - e - me - intrinsic_size - border_padding_start - border_padding_end;
            (pos, intrinsic_size)
        }
        // Only size specified: use static position
        (None, None, Some(w)) => (ms, w),
        // Nothing specified: use static position and intrinsic size
        (None, None, None) => (ms, intrinsic_size),
    }
}

/// Resolve an offset pair (e.g., left/right or top/bottom) for relative positioning.
/// If both are specified, the start value wins (per CSS 2.1).
fn resolve_offset_pair(
    start: &crate::values::LengthPercentageAuto,
    end: &crate::values::LengthPercentageAuto,
    reference: f32,
) -> f32 {
    let s = start.resolve(reference);
    let e = end.resolve(reference);

    match (s, e) {
        (Some(sv), _) => sv,     // start wins
        (None, Some(ev)) => -ev, // end is negated
        (None, None) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::LengthPercentageAuto;

    #[test]
    fn test_relative_offset_left() {
        let left = LengthPercentageAuto::px(10.0);
        let right = LengthPercentageAuto::Auto;
        assert_eq!(resolve_offset_pair(&left, &right, 800.0), 10.0);
    }

    #[test]
    fn test_relative_offset_right() {
        let left = LengthPercentageAuto::Auto;
        let right = LengthPercentageAuto::px(10.0);
        assert_eq!(resolve_offset_pair(&left, &right, 800.0), -10.0);
    }

    #[test]
    fn test_relative_offset_both_start_wins() {
        let left = LengthPercentageAuto::px(20.0);
        let right = LengthPercentageAuto::px(10.0);
        assert_eq!(resolve_offset_pair(&left, &right, 800.0), 20.0);
    }

    #[test]
    fn test_absolute_position_left_top() {
        let (x, w) = resolve_absolute_axis(
            &LengthPercentageAuto::px(10.0),
            &LengthPercentageAuto::Auto,
            &LengthPercentageAuto::px(200.0),
            &LengthPercentageAuto::px(0.0),
            &LengthPercentageAuto::px(0.0),
            0.0,
            0.0,
            800.0,
            0.0,
        );
        assert_eq!(x, 10.0);
        assert_eq!(w, 200.0);
    }

    #[test]
    fn test_absolute_position_stretch() {
        let (x, w) = resolve_absolute_axis(
            &LengthPercentageAuto::px(10.0),
            &LengthPercentageAuto::px(10.0),
            &LengthPercentageAuto::Auto,
            &LengthPercentageAuto::px(0.0),
            &LengthPercentageAuto::px(0.0),
            0.0,
            0.0,
            800.0,
            0.0,
        );
        assert_eq!(x, 10.0);
        assert_eq!(w, 780.0); // 800 - 10 - 10
    }
}
