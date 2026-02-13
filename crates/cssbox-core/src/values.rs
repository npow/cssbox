//! CSS value types and resolution.

/// A CSS length-percentage value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthPercentage {
    /// An absolute length in pixels.
    Length(f32),
    /// A percentage (0.0 to 1.0 internally, specified as 0-100%).
    Percentage(f32),
}

impl LengthPercentage {
    pub fn resolve(&self, reference: f32) -> f32 {
        match self {
            LengthPercentage::Length(px) => *px,
            LengthPercentage::Percentage(pct) => reference * pct,
        }
    }

    pub fn px(value: f32) -> Self {
        LengthPercentage::Length(value)
    }

    pub fn percent(value: f32) -> Self {
        LengthPercentage::Percentage(value / 100.0)
    }
}

impl Default for LengthPercentage {
    fn default() -> Self {
        LengthPercentage::Length(0.0)
    }
}

/// A CSS length-percentage-auto value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LengthPercentageAuto {
    Length(f32),
    Percentage(f32),
    #[default]
    Auto,
}

impl LengthPercentageAuto {
    pub fn resolve(&self, reference: f32) -> Option<f32> {
        match self {
            LengthPercentageAuto::Length(px) => Some(*px),
            LengthPercentageAuto::Percentage(pct) => Some(reference * pct),
            LengthPercentageAuto::Auto => None,
        }
    }

    pub fn resolve_or(&self, reference: f32, default: f32) -> f32 {
        self.resolve(reference).unwrap_or(default)
    }

    pub fn is_auto(&self) -> bool {
        matches!(self, LengthPercentageAuto::Auto)
    }

    pub fn px(value: f32) -> Self {
        LengthPercentageAuto::Length(value)
    }

    pub fn percent(value: f32) -> Self {
        LengthPercentageAuto::Percentage(value / 100.0)
    }
}

/// A CSS dimension that may be `none` (used for max-width/max-height).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LengthPercentageNone {
    Length(f32),
    Percentage(f32),
    #[default]
    None,
}

impl LengthPercentageNone {
    pub fn resolve(&self, reference: f32) -> Option<f32> {
        match self {
            LengthPercentageNone::Length(px) => Some(*px),
            LengthPercentageNone::Percentage(pct) => Some(reference * pct),
            LengthPercentageNone::None => None,
        }
    }

    pub fn px(value: f32) -> Self {
        LengthPercentageNone::Length(value)
    }

    pub fn percent(value: f32) -> Self {
        LengthPercentageNone::Percentage(value / 100.0)
    }
}

/// Number or auto (used for z-index, flex-grow, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum NumberOrAuto {
    Number(f32),
    #[default]
    Auto,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length_percentage_resolve() {
        let lp = LengthPercentage::px(20.0);
        assert_eq!(lp.resolve(100.0), 20.0);

        let lp = LengthPercentage::percent(50.0);
        assert_eq!(lp.resolve(200.0), 100.0);
    }

    #[test]
    fn test_length_percentage_auto_resolve() {
        let v = LengthPercentageAuto::Auto;
        assert_eq!(v.resolve(100.0), None);

        let v = LengthPercentageAuto::px(30.0);
        assert_eq!(v.resolve(100.0), Some(30.0));

        let v = LengthPercentageAuto::percent(25.0);
        assert_eq!(v.resolve(200.0), Some(50.0));
    }
}
