//! Fragment tree — the output of layout.
//!
//! Each fragment represents a positioned, sized piece of the layout output,
//! corresponding to a box tree node.

use crate::geometry::{Edges, Point, Rect, Size};
use crate::tree::NodeId;

/// A fragment produced by layout.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// The box tree node this fragment corresponds to.
    pub node: NodeId,
    /// Position relative to parent fragment.
    pub position: Point,
    /// Content box size.
    pub size: Size,
    /// Resolved padding.
    pub padding: Edges,
    /// Resolved border widths.
    pub border: Edges,
    /// Resolved margin.
    pub margin: Edges,
    /// Child fragments.
    pub children: Vec<Fragment>,
    /// Fragment kind.
    pub kind: FragmentKind,
}

/// The kind of fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentKind {
    /// A block-level box.
    Box,
    /// An anonymous block box.
    AnonymousBox,
    /// A line box (produced by inline formatting context).
    LineBox,
    /// A text run within a line box.
    TextRun,
}

impl Fragment {
    pub fn new(node: NodeId, kind: FragmentKind) -> Self {
        Self {
            node,
            position: Point::ZERO,
            size: Size::ZERO,
            padding: Edges::ZERO,
            border: Edges::ZERO,
            margin: Edges::ZERO,
            children: Vec::new(),
            kind,
        }
    }

    /// Border box rect (position + padding + border + content, relative to parent).
    pub fn border_box(&self) -> Rect {
        Rect::new(
            self.position.x,
            self.position.y,
            self.size.width + self.padding.horizontal() + self.border.horizontal(),
            self.size.height + self.padding.vertical() + self.border.vertical(),
        )
    }

    /// Margin box rect (position + margin + border + padding + content, relative to parent).
    pub fn margin_box(&self) -> Rect {
        Rect::new(
            self.position.x - self.margin.left,
            self.position.y - self.margin.top,
            self.size.width
                + self.padding.horizontal()
                + self.border.horizontal()
                + self.margin.horizontal(),
            self.size.height
                + self.padding.vertical()
                + self.border.vertical()
                + self.margin.vertical(),
        )
    }

    /// Content box rect (position offset by border + padding).
    pub fn content_box(&self) -> Rect {
        Rect::new(
            self.position.x + self.border.left + self.padding.left,
            self.position.y + self.border.top + self.padding.top,
            self.size.width,
            self.size.height,
        )
    }

    /// Compute absolute position by walking up the tree.
    /// The position is the top-left of the border box in viewport coordinates.
    pub fn absolute_position(&self, ancestors: &[&Fragment]) -> Point {
        let mut x = self.position.x;
        let mut y = self.position.y;
        for ancestor in ancestors.iter().rev() {
            x += ancestor.position.x + ancestor.border.left + ancestor.padding.left;
            y += ancestor.position.y + ancestor.border.top + ancestor.padding.top;
        }
        Point::new(x, y)
    }
}

/// The result of a layout computation.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// The root fragment.
    pub root: Fragment,
}

impl LayoutResult {
    /// Find a fragment by node ID (depth-first search).
    pub fn find_fragment(&self, node: NodeId) -> Option<&Fragment> {
        find_in_fragment(&self.root, node)
    }

    /// Get the layout output for a node (the bounding client rect equivalent).
    pub fn get_layout(&self, node: NodeId) -> Option<LayoutOutput> {
        // We need the absolute position, which requires ancestors
        let mut path = Vec::new();
        if find_path_to_node(&self.root, node, &mut path) {
            let fragment = path.last().unwrap();
            let ancestors: Vec<&Fragment> = path[..path.len() - 1].to_vec();
            let abs_pos = fragment.absolute_position(&ancestors);
            Some(LayoutOutput {
                position: abs_pos,
                size: fragment.size,
                padding: fragment.padding,
                border: fragment.border,
                margin: fragment.margin,
            })
        } else {
            None
        }
    }

    /// Get the bounding client rect for a node.
    pub fn bounding_rect(&self, node: NodeId) -> Option<Rect> {
        self.get_layout(node).map(|l| {
            Rect::new(
                l.position.x,
                l.position.y,
                l.size.width + l.padding.horizontal() + l.border.horizontal(),
                l.size.height + l.padding.vertical() + l.border.vertical(),
            )
        })
    }
}

/// Layout output for a single node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutOutput {
    /// Absolute position of the border box (x, y).
    pub position: Point,
    /// Content box size.
    pub size: Size,
    /// Resolved padding.
    pub padding: Edges,
    /// Resolved border widths.
    pub border: Edges,
    /// Resolved margin.
    pub margin: Edges,
}

fn find_in_fragment(fragment: &Fragment, node: NodeId) -> Option<&Fragment> {
    if fragment.node == node {
        return Some(fragment);
    }
    for child in &fragment.children {
        if let Some(found) = find_in_fragment(child, node) {
            return Some(found);
        }
    }
    None
}

fn find_path_to_node<'a>(
    fragment: &'a Fragment,
    node: NodeId,
    path: &mut Vec<&'a Fragment>,
) -> bool {
    path.push(fragment);
    if fragment.node == node {
        return true;
    }
    for child in &fragment.children {
        if find_path_to_node(child, node, path) {
            return true;
        }
    }
    path.pop();
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_border_box() {
        let mut f = Fragment::new(NodeId(0), FragmentKind::Box);
        f.position = Point::new(10.0, 20.0);
        f.size = Size::new(100.0, 50.0);
        f.padding = Edges::all(5.0);
        f.border = Edges::all(1.0);

        let bb = f.border_box();
        assert_eq!(bb.width, 112.0); // 100 + 5*2 + 1*2
        assert_eq!(bb.height, 62.0); // 50 + 5*2 + 1*2
    }
}
