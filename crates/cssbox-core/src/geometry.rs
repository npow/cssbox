//! Geometric primitives for layout computation.

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn offset(self, dx: f32, dy: f32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

/// A 2D size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_point_size(point: Point, size: Size) -> Self {
        Self {
            x: point.x,
            y: point.y,
            width: size.width,
            height: size.height,
        }
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }
}

/// Edge values (top, right, bottom, left) — used for margin, padding, border.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Total horizontal extent (left + right).
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Total vertical extent (top + bottom).
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Available space constraint for layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AvailableSpace {
    /// A definite size in pixels.
    Definite(f32),
    /// Size determined by content (shrink-to-fit).
    MinContent,
    /// Size determined by content with no wrapping.
    MaxContent,
}

impl AvailableSpace {
    pub fn to_definite(self) -> Option<f32> {
        match self {
            AvailableSpace::Definite(v) => Some(v),
            _ => None,
        }
    }

    pub fn unwrap_or(self, default: f32) -> f32 {
        match self {
            AvailableSpace::Definite(v) => v,
            _ => default,
        }
    }
}

impl Default for AvailableSpace {
    fn default() -> Self {
        AvailableSpace::Definite(0.0)
    }
}

/// Size constraints with optional min/max.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeConstraint {
    pub available: AvailableSpace,
    pub min: f32,
    pub max: f32,
}

impl SizeConstraint {
    pub fn new(available: AvailableSpace) -> Self {
        Self {
            available,
            min: 0.0,
            max: f32::INFINITY,
        }
    }

    pub fn clamp(&self, value: f32) -> f32 {
        value.max(self.min).min(self.max)
    }
}

impl Default for SizeConstraint {
    fn default() -> Self {
        Self {
            available: AvailableSpace::Definite(0.0),
            min: 0.0,
            max: f32::INFINITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_offset() {
        let p = Point::new(10.0, 20.0);
        let q = p.offset(5.0, -3.0);
        assert_eq!(q, Point::new(15.0, 17.0));
    }

    #[test]
    fn test_edges_horizontal_vertical() {
        let e = Edges::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(e.horizontal(), 6.0);
        assert_eq!(e.vertical(), 4.0);
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(r.contains(Point::new(50.0, 30.0)));
        assert!(!r.contains(Point::new(5.0, 5.0)));
    }

    #[test]
    fn test_size_constraint_clamp() {
        let c = SizeConstraint {
            available: AvailableSpace::Definite(100.0),
            min: 20.0,
            max: 80.0,
        };
        assert_eq!(c.clamp(50.0), 50.0);
        assert_eq!(c.clamp(10.0), 20.0);
        assert_eq!(c.clamp(100.0), 80.0);
    }
}
