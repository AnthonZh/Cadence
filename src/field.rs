pub const STEPS_PER_YARD: f32 = 1.6;
pub const LENGTH_YARDS: f32 = 100.0;
pub const WIDTH_YARDS: f32 = 160.0 / 3.0;
pub const LENGTH_STEPS: f32 = LENGTH_YARDS * STEPS_PER_YARD;
pub const WIDTH_STEPS: f32 = WIDTH_YARDS * STEPS_PER_YARD;
pub const FRONT_HASH_YARDS: f32 = 20.0;
pub const BACK_HASH_YARDS: f32 = WIDTH_YARDS - FRONT_HASH_YARDS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalReference {
    FrontSideline,
    FrontHash,
    BackHash,
    BackSideline,
}

impl VerticalReference {
    pub const ALL: [Self; 4] = [
        Self::FrontSideline,
        Self::FrontHash,
        Self::BackHash,
        Self::BackSideline,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::FrontSideline => "Front sideline",
            Self::FrontHash => "Front hash",
            Self::BackHash => "Back hash",
            Self::BackSideline => "Back sideline",
        }
    }

    pub const fn yards(self) -> f32 {
        match self {
            Self::FrontSideline => 0.0,
            Self::FrontHash => FRONT_HASH_YARDS,
            Self::BackHash => BACK_HASH_YARDS,
            Self::BackSideline => WIDTH_YARDS,
        }
    }
}

pub fn steps_to_yards(steps: f32) -> f32 {
    steps / STEPS_PER_YARD
}

pub fn yards_to_steps(yards: f32) -> f32 {
    yards * STEPS_PER_YARD
}

pub fn nearest_vertical_reference(y_steps: f32) -> VerticalReference {
    let y_yards = steps_to_yards(y_steps);
    VerticalReference::ALL
        .into_iter()
        .min_by(|a, b| {
            (a.yards() - y_yards)
                .abs()
                .total_cmp(&(b.yards() - y_yards).abs())
        })
        .unwrap()
}

pub fn nearest_yard_line(x_steps: f32) -> f32 {
    (steps_to_yards(x_steps) / 5.0).round().clamp(0.0, 20.0) * 5.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_field_conversions_are_exact() {
        assert_eq!(steps_to_yards(LENGTH_STEPS), 100.0);
        assert!((steps_to_yards(WIDTH_STEPS) - 53.333_332).abs() < 0.001);
        assert_eq!(yards_to_steps(5.0), 8.0);
        assert_eq!(nearest_yard_line(yards_to_steps(37.0)), 35.0);
        assert_eq!(
            nearest_vertical_reference(yards_to_steps(21.0)),
            VerticalReference::FrontHash
        );
        assert!((BACK_HASH_YARDS - 33.333_332).abs() < 0.001);
    }
}
