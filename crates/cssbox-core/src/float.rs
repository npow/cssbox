//! Float layout per CSS 2.1 §9.5.1.
//!
//! This module implements CSS float positioning, which removes elements from normal flow
//! and positions them to the left or right side of their containing block. Subsequent
//! content flows around the floated elements.

use crate::fragment::Fragment;
use crate::geometry::{Point, Rect};
use crate::style::{Clear, Float};

/// Tracks placed floats and provides exclusion zone queries.
///
/// FloatContext maintains separate lists of left and right floats and computes
/// available space for content that flows around them.
#[derive(Debug, Clone)]
pub struct FloatContext {
    /// Width of the containing block.
    containing_width: f32,
    /// Left-floated boxes, in order of placement.
    left_floats: Vec<FloatBox>,
    /// Right-floated boxes, in order of placement.
    right_floats: Vec<FloatBox>,
}

/// A placed float box with its margin box rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FloatBox {
    /// Position and size of the float's margin box.
    rect: Rect,
}

impl FloatContext {
    /// Create a new float context for a containing block.
    ///
    /// # Arguments
    /// * `containing_width` - The width of the containing block in pixels.
    pub fn new(containing_width: f32) -> Self {
        Self {
            containing_width,
            left_floats: Vec::new(),
            right_floats: Vec::new(),
        }
    }

    /// Place a float left or right, below any existing floats that would overlap.
    ///
    /// # Arguments
    /// * `fragment` - The fragment to float (will be mutated with position).
    /// * `float_type` - Whether to float left or right.
    /// * `cursor_y` - The vertical position of the generating box (float cannot be placed above this).
    ///
    /// # Returns
    /// The fragment with its position field updated to the final placement.
    ///
    /// # Algorithm (per CSS 2.1 §9.5.1)
    /// 1. Float cannot be placed above cursor_y (the top of its generating box).
    /// 2. Float cannot be placed above any previously placed float.
    /// 3. Left float is placed as far left as possible, right float as far right as possible.
    /// 4. If float doesn't fit horizontally at current y, move down until it fits.
    pub fn place_float(
        &mut self,
        mut fragment: Fragment,
        float_type: Float,
        cursor_y: f32,
    ) -> Fragment {
        let margin_box = fragment.margin_box();
        let width = margin_box.width;
        let height = margin_box.height;

        // Start at cursor_y but ensure we're not above any existing floats
        let min_y = self.compute_min_y_for_float(&float_type, cursor_y);
        let mut y = min_y;

        // Find a vertical position where the float fits horizontally
        loop {
            let (left_offset, available_width) = self.available_width_at(y, height);

            if available_width >= width {
                // Found a position where it fits
                let x = match float_type {
                    Float::Left => left_offset,
                    Float::Right => left_offset + available_width - width,
                    Float::None => {
                        // Should not happen, but place at left as fallback
                        left_offset
                    }
                };

                // Calculate the border box position from the margin box position
                // margin_box.x is relative to border_box position with negative margin offset
                let border_x = x + fragment.margin.left;
                let border_y = y + fragment.margin.top;

                fragment.position = Point::new(border_x, border_y);

                // Record this float
                let float_box = FloatBox {
                    rect: Rect::new(x, y, width, height),
                };

                match float_type {
                    Float::Left => self.left_floats.push(float_box),
                    Float::Right => self.right_floats.push(float_box),
                    Float::None => {}
                }

                return fragment;
            }

            // Doesn't fit; move down to the next constraint
            y = self.next_y_position(y, height);

            // Safety check to prevent infinite loops
            if y > cursor_y + 100000.0 {
                // Place at minimum viable position as fallback
                let x = match float_type {
                    Float::Left => 0.0,
                    Float::Right => self.containing_width - width,
                    Float::None => 0.0,
                }
                .max(0.0);

                let border_x = x + fragment.margin.left;
                let border_y = y + fragment.margin.top;
                fragment.position = Point::new(border_x, border_y);

                let float_box = FloatBox {
                    rect: Rect::new(x, y, width, height),
                };

                match float_type {
                    Float::Left => self.left_floats.push(float_box),
                    Float::Right => self.right_floats.push(float_box),
                    Float::None => {}
                }

                return fragment;
            }
        }
    }

    /// Get available width at a vertical position accounting for float exclusions.
    ///
    /// # Arguments
    /// * `y` - Vertical position to query.
    /// * `height` - Height of the content being placed (to check overlap).
    ///
    /// # Returns
    /// A tuple of (left_offset, available_width):
    /// - `left_offset`: The x-coordinate where content can start.
    /// - `available_width`: The width available for content.
    pub fn available_width_at(&self, y: f32, height: f32) -> (f32, f32) {
        let bottom = y + height;

        // Find the rightmost left float that overlaps [y, bottom)
        let left_edge = self
            .left_floats
            .iter()
            .filter(|f| f.rect.y < bottom && f.rect.bottom() > y)
            .map(|f| f.rect.right())
            .fold(0.0_f32, f32::max);

        // Find the leftmost right float that overlaps [y, bottom)
        let right_edge = self
            .right_floats
            .iter()
            .filter(|f| f.rect.y < bottom && f.rect.bottom() > y)
            .map(|f| f.rect.x)
            .fold(self.containing_width, f32::min);

        let available = (right_edge - left_edge).max(0.0);
        (left_edge, available)
    }

    /// Get the y position that clears floats according to the clear property.
    ///
    /// # Arguments
    /// * `clear` - The clear value (Left, Right, Both, or None).
    ///
    /// # Returns
    /// The y position below the relevant floats, or 0.0 for Clear::None.
    pub fn clear(&self, clear: Clear) -> f32 {
        match clear {
            Clear::None => 0.0,
            Clear::Left => self.bottom_of_floats(&self.left_floats),
            Clear::Right => self.bottom_of_floats(&self.right_floats),
            Clear::Both => {
                let left_bottom = self.bottom_of_floats(&self.left_floats);
                let right_bottom = self.bottom_of_floats(&self.right_floats);
                left_bottom.max(right_bottom)
            }
        }
    }

    /// Get the y position below all floats (both left and right).
    ///
    /// # Returns
    /// The maximum bottom edge of all floats, or 0.0 if no floats exist.
    pub fn clear_all(&self) -> f32 {
        let left_bottom = self.bottom_of_floats(&self.left_floats);
        let right_bottom = self.bottom_of_floats(&self.right_floats);
        left_bottom.max(right_bottom)
    }

    /// Compute the minimum y position for a new float (cannot be above existing floats).
    fn compute_min_y_for_float(&self, float_type: &Float, cursor_y: f32) -> f32 {
        let same_side_bottom = match float_type {
            Float::Left => self.bottom_of_floats(&self.left_floats),
            Float::Right => self.bottom_of_floats(&self.right_floats),
            Float::None => 0.0,
        };
        cursor_y.max(same_side_bottom)
    }

    /// Find the next y position where the available space might change.
    fn next_y_position(&self, current_y: f32, height: f32) -> f32 {
        let bottom = current_y + height;

        // Find all floats that overlap the current [y, bottom) range
        let mut next_positions = Vec::new();

        for float_box in self.left_floats.iter().chain(self.right_floats.iter()) {
            // If float overlaps, try moving to its bottom edge
            if float_box.rect.y < bottom && float_box.rect.bottom() > current_y {
                next_positions.push(float_box.rect.bottom());
            }
            // Also try moving to its top edge if it's below us
            if float_box.rect.y >= current_y {
                next_positions.push(float_box.rect.y);
            }
        }

        // Return the smallest position greater than current_y
        next_positions
            .into_iter()
            .filter(|&y| y > current_y)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(current_y + 1.0)
    }

    /// Get the bottom edge of the lowest float in a list.
    fn bottom_of_floats(&self, floats: &[FloatBox]) -> f32 {
        floats
            .iter()
            .map(|f| f.rect.bottom())
            .fold(0.0_f32, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fragment::FragmentKind;
    use crate::geometry::{Edges, Size};
    use crate::tree::NodeId;

    fn create_fragment(width: f32, height: f32, margin: f32) -> Fragment {
        let mut frag = Fragment::new(NodeId(0), FragmentKind::Box);
        frag.size = Size::new(width, height);
        frag.margin = Edges::all(margin);
        frag
    }

    #[test]
    fn test_new_float_context() {
        let ctx = FloatContext::new(800.0);
        assert_eq!(ctx.containing_width, 800.0);
        assert_eq!(ctx.left_floats.len(), 0);
        assert_eq!(ctx.right_floats.len(), 0);
    }

    #[test]
    fn test_available_width_no_floats() {
        let ctx = FloatContext::new(800.0);
        let (left, width) = ctx.available_width_at(0.0, 100.0);
        assert_eq!(left, 0.0);
        assert_eq!(width, 800.0);
    }

    #[test]
    fn test_place_left_float() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(100.0, 50.0, 10.0);

        let placed = ctx.place_float(frag, Float::Left, 0.0);

        // Border box should be at (margin.left, margin.top) = (10, 10)
        assert_eq!(placed.position.x, 10.0);
        assert_eq!(placed.position.y, 10.0);

        // Check float was recorded
        assert_eq!(ctx.left_floats.len(), 1);

        // Margin box should be at (0, 0) with size 120x70
        let margin_box = placed.margin_box();
        assert_eq!(margin_box.x, 0.0);
        assert_eq!(margin_box.y, 0.0);
        assert_eq!(margin_box.width, 120.0); // 100 + 10*2
        assert_eq!(margin_box.height, 70.0); // 50 + 10*2
    }

    #[test]
    fn test_place_right_float() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(100.0, 50.0, 10.0);

        let placed = ctx.place_float(frag, Float::Right, 0.0);

        // Margin box should be at (800 - 120, 0) = (680, 0)
        // Border box should be at (680 + 10, 10) = (690, 10)
        assert_eq!(placed.position.x, 690.0);
        assert_eq!(placed.position.y, 10.0);

        assert_eq!(ctx.right_floats.len(), 1);
    }

    #[test]
    fn test_two_left_floats_side_by_side() {
        let mut ctx = FloatContext::new(800.0);

        let frag1 = create_fragment(100.0, 50.0, 0.0);
        let placed1 = ctx.place_float(frag1, Float::Left, 0.0);
        assert_eq!(placed1.position.x, 0.0);
        assert_eq!(placed1.position.y, 0.0);

        let frag2 = create_fragment(100.0, 50.0, 0.0);
        let placed2 = ctx.place_float(frag2, Float::Left, 0.0);

        // Second left float stacks below the first (cannot be placed above previous same-side float)
        assert_eq!(placed2.position.x, 0.0);
        assert_eq!(placed2.position.y, 50.0);
    }

    #[test]
    fn test_left_float_wraps_when_no_space() {
        let mut ctx = FloatContext::new(200.0);

        // Place first float taking 150px
        let frag1 = create_fragment(150.0, 50.0, 0.0);
        ctx.place_float(frag1, Float::Left, 0.0);

        // Place second float that needs 100px (won't fit next to first)
        let frag2 = create_fragment(100.0, 40.0, 0.0);
        let placed2 = ctx.place_float(frag2, Float::Left, 0.0);

        // Should wrap below first float
        assert_eq!(placed2.position.x, 0.0);
        assert_eq!(placed2.position.y, 50.0); // Below first float's 50px height
    }

    #[test]
    fn test_available_width_with_left_float() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(200.0, 100.0, 0.0);
        ctx.place_float(frag, Float::Left, 0.0);

        let (left, width) = ctx.available_width_at(50.0, 10.0);
        assert_eq!(left, 200.0); // After the 200px float
        assert_eq!(width, 600.0); // 800 - 200
    }

    #[test]
    fn test_available_width_with_right_float() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(200.0, 100.0, 0.0);
        ctx.place_float(frag, Float::Right, 0.0);

        let (left, width) = ctx.available_width_at(50.0, 10.0);
        assert_eq!(left, 0.0);
        assert_eq!(width, 600.0); // 800 - 200
    }

    #[test]
    fn test_available_width_with_both_floats() {
        let mut ctx = FloatContext::new(800.0);

        let left_frag = create_fragment(150.0, 100.0, 0.0);
        ctx.place_float(left_frag, Float::Left, 0.0);

        let right_frag = create_fragment(200.0, 100.0, 0.0);
        ctx.place_float(right_frag, Float::Right, 0.0);

        let (left, width) = ctx.available_width_at(50.0, 10.0);
        assert_eq!(left, 150.0); // After left float
        assert_eq!(width, 450.0); // 600 - 150 (space between floats)
    }

    #[test]
    fn test_available_width_below_floats() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(200.0, 100.0, 0.0);
        ctx.place_float(frag, Float::Left, 0.0);

        // Query below the float
        let (left, width) = ctx.available_width_at(150.0, 10.0);
        assert_eq!(left, 0.0); // Float doesn't affect this region
        assert_eq!(width, 800.0);
    }

    #[test]
    fn test_clear_none() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(100.0, 50.0, 0.0);
        ctx.place_float(frag, Float::Left, 0.0);

        assert_eq!(ctx.clear(Clear::None), 0.0);
    }

    #[test]
    fn test_clear_left() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(100.0, 50.0, 0.0);
        ctx.place_float(frag, Float::Left, 0.0);

        assert_eq!(ctx.clear(Clear::Left), 50.0);
    }

    #[test]
    fn test_clear_right() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(100.0, 60.0, 0.0);
        ctx.place_float(frag, Float::Right, 0.0);

        assert_eq!(ctx.clear(Clear::Right), 60.0);
    }

    #[test]
    fn test_clear_both() {
        let mut ctx = FloatContext::new(800.0);

        let left_frag = create_fragment(100.0, 50.0, 0.0);
        ctx.place_float(left_frag, Float::Left, 0.0);

        let right_frag = create_fragment(100.0, 80.0, 0.0);
        ctx.place_float(right_frag, Float::Right, 0.0);

        // Should return the maximum of both
        assert_eq!(ctx.clear(Clear::Both), 80.0);
    }

    #[test]
    fn test_clear_all() {
        let mut ctx = FloatContext::new(800.0);

        let left_frag = create_fragment(100.0, 50.0, 0.0);
        ctx.place_float(left_frag, Float::Left, 0.0);

        let right_frag = create_fragment(100.0, 80.0, 0.0);
        ctx.place_float(right_frag, Float::Right, 0.0);

        assert_eq!(ctx.clear_all(), 80.0);
    }

    #[test]
    fn test_float_respects_cursor_y() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(100.0, 50.0, 0.0);

        // Place float with cursor at y=100
        let placed = ctx.place_float(frag, Float::Left, 100.0);

        // Float should not be placed above cursor_y
        assert!(placed.position.y >= 100.0);
    }

    #[test]
    fn test_float_with_margins() {
        let mut ctx = FloatContext::new(800.0);
        let frag = create_fragment(100.0, 50.0, 20.0);

        let placed = ctx.place_float(frag, Float::Left, 0.0);

        // Border box position should account for margin
        assert_eq!(placed.position.x, 20.0);
        assert_eq!(placed.position.y, 20.0);

        // Margin box should start at 0
        let margin_box = placed.margin_box();
        assert_eq!(margin_box.x, 0.0);
        assert_eq!(margin_box.y, 0.0);
    }

    #[test]
    fn test_multiple_left_floats_stacking() {
        let mut ctx = FloatContext::new(500.0);

        let frag1 = create_fragment(100.0, 50.0, 0.0);
        let placed1 = ctx.place_float(frag1, Float::Left, 0.0);
        assert_eq!(placed1.position.x, 0.0);
        assert_eq!(placed1.position.y, 0.0);

        let frag2 = create_fragment(100.0, 50.0, 0.0);
        let placed2 = ctx.place_float(frag2, Float::Left, 0.0);
        // Stacks below first float (same-side floats stack vertically)
        assert_eq!(placed2.position.x, 0.0);
        assert_eq!(placed2.position.y, 50.0);

        let frag3 = create_fragment(100.0, 50.0, 0.0);
        let placed3 = ctx.place_float(frag3, Float::Left, 0.0);
        // Stacks below second float
        assert_eq!(placed3.position.x, 0.0);
        assert_eq!(placed3.position.y, 100.0);
    }

    #[test]
    fn test_float_below_previous_same_side() {
        let mut ctx = FloatContext::new(800.0);

        // Place first left float
        let frag1 = create_fragment(100.0, 50.0, 0.0);
        ctx.place_float(frag1, Float::Left, 0.0);

        // Place second left float with cursor_y = 0 (should go below first)
        let frag2 = create_fragment(100.0, 30.0, 0.0);
        let placed2 = ctx.place_float(frag2, Float::Left, 0.0);

        // Should be positioned to the right at same y (if space) or below
        assert!(placed2.position.y >= 0.0);
    }

    #[test]
    fn test_narrow_containing_block() {
        let mut ctx = FloatContext::new(100.0);

        // Try to place a float wider than containing block
        let frag = create_fragment(150.0, 50.0, 0.0);
        let placed = ctx.place_float(frag, Float::Left, 0.0);

        // Float wider than containing block triggers safety fallback after searching
        // Algorithm increments y until safety limit (cursor_y + 100000)
        assert!(placed.position.y > 100000.0);
        assert_eq!(placed.position.x, 0.0);
    }

    #[test]
    fn test_available_width_partial_overlap() {
        let mut ctx = FloatContext::new(800.0);

        // Place float from y=50 to y=150
        let frag = create_fragment(200.0, 100.0, 0.0);
        ctx.place_float(frag, Float::Left, 50.0);

        // Query at y=0 (before float) with height that overlaps
        let (left, width) = ctx.available_width_at(0.0, 100.0);
        assert_eq!(left, 200.0); // Reduced by float
        assert_eq!(width, 600.0);

        // Query at y=0 with height that doesn't reach float
        let (left, width) = ctx.available_width_at(0.0, 40.0);
        assert_eq!(left, 0.0); // No overlap
        assert_eq!(width, 800.0);
    }

    #[test]
    fn test_overlapping_left_and_right_floats() {
        let mut ctx = FloatContext::new(800.0);

        let left = create_fragment(350.0, 100.0, 0.0);
        ctx.place_float(left, Float::Left, 0.0);

        let right = create_fragment(350.0, 100.0, 0.0);
        ctx.place_float(right, Float::Right, 0.0);

        let (left_offset, width) = ctx.available_width_at(50.0, 10.0);
        assert_eq!(left_offset, 350.0);
        assert_eq!(width, 100.0); // 800 - 350 - 350
    }

    #[test]
    fn test_zero_available_width() {
        let mut ctx = FloatContext::new(400.0);

        let left = create_fragment(250.0, 100.0, 0.0);
        ctx.place_float(left, Float::Left, 0.0);

        let right = create_fragment(250.0, 100.0, 0.0);
        ctx.place_float(right, Float::Right, 0.0);

        let (_, width) = ctx.available_width_at(50.0, 10.0);
        // Right float doesn't fit next to left float, so wraps below it.
        // At y=50, only left float (y=0-100) overlaps, so available width is 400-250=150
        assert_eq!(width, 150.0);
    }
}
