#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ValueDisplayScale {
    /// `0.0` is linear. `1.0` maps `0.5 -> 0.75`.
    pub skew_factor: f32,
}

impl Default for ValueDisplayScale {
    fn default() -> Self {
        Self { skew_factor: 0.0 }
    }
}

impl ValueDisplayScale {
    const SKEW_UNIT_EXPONENT: f64 = 0.415_037_499_278_843_8;
    pub const MAX_SKEW_FACTOR: f32 = 5.0;

    pub fn exponent(self) -> f64 {
        let skew = self.skew_factor.clamp(0.0, Self::MAX_SKEW_FACTOR) as f64;
        Self::SKEW_UNIT_EXPONENT.powf(skew)
    }

    pub fn sample_to_display(self, sample_value: f64) -> f64 {
        if sample_value == 0.0 {
            return 0.0;
        }

        sample_value.signum() * sample_value.abs().powf(self.exponent())
    }

    pub fn display_to_sample(self, display_value: f64) -> f64 {
        if display_value == 0.0 {
            return 0.0;
        }

        display_value.signum() * display_value.abs().powf(1.0 / self.exponent())
    }
}

#[cfg(test)]
mod tests {
    use super::ValueDisplayScale;

    #[test]
    fn endpoints_and_zero_stay_fixed() {
        let scale = ValueDisplayScale { skew_factor: 1.0 };

        assert_eq!(scale.sample_to_display(-1.0), -1.0);
        assert_eq!(scale.sample_to_display(0.0), 0.0);
        assert_eq!(scale.sample_to_display(1.0), 1.0);
    }

    #[test]
    fn positive_and_negative_sides_are_symmetric() {
        let scale = ValueDisplayScale { skew_factor: 0.8 };
        let positive = scale.sample_to_display(0.3);
        let negative = scale.sample_to_display(-0.3);

        assert!((positive + negative).abs() < 1e-9);
    }

    #[test]
    fn skew_expands_low_amplitudes() {
        let linear = ValueDisplayScale::default();
        let skewed = ValueDisplayScale { skew_factor: 1.0 };

        assert_eq!(linear.sample_to_display(0.5), 0.5);
        assert!((skewed.sample_to_display(0.5) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn larger_skew_values_expand_more() {
        let skewed_a = ValueDisplayScale { skew_factor: 1.0 };
        let skewed_b = ValueDisplayScale { skew_factor: 5.0 };

        assert!(skewed_b.sample_to_display(0.5) > skewed_a.sample_to_display(0.5));
    }

    #[test]
    fn inverse_round_trip_is_stable() {
        let scale = ValueDisplayScale { skew_factor: 0.65 };

        for sample_value in [-2.0, -1.5, -1.0, -0.5, -0.1, 0.0, 0.1, 0.5, 1.0, 1.5, 2.0] {
            let display_value = scale.sample_to_display(sample_value);
            let round_trip = scale.display_to_sample(display_value);

            assert!((sample_value - round_trip).abs() < 1e-9);
        }
    }

    #[test]
    fn linear_scale_preserves_out_of_range_values() {
        let scale = ValueDisplayScale::default();

        assert_eq!(scale.sample_to_display(-2.0), -2.0);
        assert_eq!(scale.sample_to_display(2.0), 2.0);
        assert_eq!(scale.display_to_sample(-1.5), -1.5);
        assert_eq!(scale.display_to_sample(1.5), 1.5);
    }

    #[test]
    fn skewed_scale_is_monotonic_outside_full_scale() {
        let scale = ValueDisplayScale { skew_factor: 1.0 };
        let inputs = [-2.0, -1.5, -1.0, -0.5, 0.5, 1.0, 1.5, 2.0];
        let outputs: Vec<_> = inputs
            .into_iter()
            .map(|value| scale.sample_to_display(value))
            .collect();

        assert!(outputs.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
