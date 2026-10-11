use crate::{
    audio::sample,
    model::ruler::{
        TickType, ValueDisplayScale, sample_value_to_screen_y, screen_y_to_sample_value,
    },
    rect,
};

/// Floor on amplitude so we never emit ticks that are visually closer to the zero crossing
/// than half a pixel — those would collapse into the mirror tick on the other side and produce
/// unreadable label stacks.
const MIN_VISIBLE_AMP: f64 = 1e-7;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DbTick {
    /// dB value this tick represents (typically <= 0, may be positive if amp > 1.0).
    pub db: f64,
    /// Signed amplitude this tick corresponds to. Each dB value yields up to two ticks
    /// (one on the +amp side, one on the -amp side) when both fall inside the visible range.
    pub sample_value: f64,
    pub screen_y: f32,
    pub tick_type: TickType,
}

#[derive(Debug, Clone, Default)]
pub struct DbLattice {
    pub ticks: Vec<DbTick>,
    /// Fine candidates retained for filling readable gaps between the coarse ticks.
    pub label_candidates: Vec<DbTick>,
    /// Finest candidate cadence; coarse tick marks keep their own cadence below.
    pub finest_label_step_db: f64,
    pub label_step_db: f64,
    /// Cadence of `TickType::Mid` ticks, when the ladder entry warrants one.
    pub mid_step_db: Option<f64>,
    /// Cadence of `TickType::Small` ticks.
    pub minor_step_db: f64,
}

const FINE_STEP_DB: f64 = 1.0;
const MIN_DB: f64 = -144.0;
const MAX_DB: f64 = 12.0;
const LADDER_DB: &[f64] = &[1.0, 2.0, 3.0, 6.0, 12.0, 24.0, 48.0];
pub(crate) const FRACTIONAL_STEPS_DB: &[f64] = &[
    0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.005, 0.002, 0.001, 0.0005, 0.0002, 0.0001, 0.00005, 0.00002,
    0.00001, 0.000005, 0.000002, 0.000001,
];
const FRACTIONAL_BAND_HEIGHT: f32 = 24.0;
const MAX_FRACTIONAL_BANDS: usize = 64;
const FRACTIONAL_INTERVALS_PER_SIDE: usize = 6;

impl DbLattice {
    pub fn compute_ticks(
        &mut self,
        val_range: sample::ValRange<f64>,
        screen_rect: rect::Rect,
        nr_pixels_per_tick: f32,
        display_scale: ValueDisplayScale,
    ) -> anyhow::Result<()> {
        self.ticks.clear();
        self.label_candidates.clear();
        self.finest_label_step_db = 1.0;
        self.label_step_db = LADDER_DB[0];
        self.mid_step_db = None;
        self.minor_step_db = LADDER_DB[0];

        if screen_rect.height() <= 0.0 || val_range.is_empty() {
            anyhow::bail!("value range or screen rect invalid, cannot draw dB lattice");
        }

        // Cap the deepest dB we'll consider at the per-pixel amplitude resolution. A linear
        // estimate is too pessimistic under skew (low amplitudes get blown up near zero), so
        // we probe the actual screen mapping one pixel above the zero-amplitude row and use
        // *that* amplitude as the floor. Equivalent to the linear estimate when skew = 0.
        let center_y = sample_value_to_screen_y(0.0, val_range, screen_rect, display_scale)
            .unwrap_or(screen_rect.center().y)
            .clamp(screen_rect.top(), screen_rect.bottom());
        let probe_y = (center_y - 1.0).max(screen_rect.top());
        let near_amp = screen_y_to_sample_value(probe_y, val_range, screen_rect, display_scale)
            .map(|v| v.abs())
            .unwrap_or(MIN_VISIBLE_AMP)
            .max(MIN_VISIBLE_AMP);
        let min_visible_db = (20.0 * near_amp.log10()).max(MIN_DB);
        let visible_samples = sample::ValRange {
            min: display_scale.axis_to_sample(val_range.min),
            max: display_scale.axis_to_sample(val_range.max),
        };

        // Enumerate candidates at the finest cadence. For each dB value we test both +amp and
        // -amp because the ruler is symmetric around zero amplitude.
        let mut candidates: Vec<DbTick> = Vec::new();
        let mut db = MAX_DB;
        while db >= min_visible_db - FINE_STEP_DB * 0.5 {
            let db_q = quantize_to_step(db, FINE_STEP_DB);
            let amp = 10.0_f64.powf(db_q / 20.0);
            for sign in [1.0_f64, -1.0] {
                let sample_value = sign * amp;
                if sample_value < visible_samples.min || sample_value > visible_samples.max {
                    continue;
                }
                if let Some(screen_y) =
                    sample_value_to_screen_y(sample_value, val_range, screen_rect, display_scale)
                    && screen_rect.contains_y(screen_y)
                {
                    candidates.push(DbTick {
                        db: db_q,
                        sample_value,
                        screen_y,
                        tick_type: TickType::Small,
                    });
                }
            }
            db -= FINE_STEP_DB;
        }

        candidates.sort_by(|a, b| {
            a.screen_y
                .partial_cmp(&b.screen_y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Pick the smallest ladder cadence where every adjacent pair of would-be label ticks is
        // at least `nr_pixels_per_tick` apart on screen.
        let label_step_db = pick_step(&candidates, nr_pixels_per_tick);
        // Minor cadence below label cadence — used for short ticks.
        let minor_step_db = minor_step_for(label_step_db);
        let mid_step_db = mid_step_for(label_step_db);

        self.label_step_db = label_step_db;
        self.mid_step_db = mid_step_db;
        self.minor_step_db = minor_step_db;
        for tick in &candidates {
            let tick_type = if is_multiple_of_db(tick.db, label_step_db) {
                TickType::Big
            } else if mid_step_db.is_some_and(|s| is_multiple_of_db(tick.db, s)) {
                TickType::Mid
            } else {
                TickType::Small
            };
            let tick = DbTick { tick_type, ..*tick };
            self.label_candidates.push(tick);
            if is_multiple_of_db(tick.db, minor_step_db) {
                self.ticks.push(tick);
            }
        }

        self.add_fractional_candidates(val_range, screen_rect, display_scale);
        self.label_candidates.sort_by(|a, b| {
            a.sample_value
                .signum()
                .total_cmp(&b.sample_value.signum())
                .then(a.db.total_cmp(&b.db))
        });
        self.label_candidates.dedup_by(|a, b| {
            a.sample_value.signum() == b.sample_value.signum() && (a.db - b.db).abs() < 1e-9
        });
        self.label_candidates
            .sort_by(|a, b| a.screen_y.total_cmp(&b.screen_y));

        Ok(())
    }

    fn add_fractional_candidates(
        &mut self,
        range: sample::ValRange<f64>,
        screen: rect::Rect,
        scale: ValueDisplayScale,
    ) {
        let bands = fractional_band_count(screen.height());
        for band in 0..bands {
            let top = screen.top() + screen.height() * band as f32 / bands as f32;
            let bottom = screen.top() + screen.height() * (band + 1) as f32 / bands as f32;
            let (Some(a), Some(b)) = (
                screen_y_to_sample_value(top, range, screen, scale),
                screen_y_to_sample_value(bottom, range, screen, scale),
            ) else {
                continue;
            };
            let low = a.min(b);
            let high = a.max(b);
            if low >= high {
                continue;
            }
            for (sign, min_amp, max_amp) in [
                (1.0, low.max(0.0), high.max(0.0)),
                (-1.0, -high.min(0.0), -low.min(0.0)),
            ] {
                if max_amp <= MIN_VISIBLE_AMP {
                    continue;
                }
                let min_db = (20.0 * min_amp.max(MIN_VISIBLE_AMP).log10()).max(MIN_DB);
                let max_db = (20.0 * max_amp.log10()).min(MAX_DB);
                if min_db >= max_db {
                    continue;
                }
                let target_step = (max_db - min_db) / FRACTIONAL_INTERVALS_PER_SIDE as f64;
                let Some(step) = FRACTIONAL_STEPS_DB
                    .iter()
                    .rev()
                    .copied()
                    .find(|step| *step >= target_step)
                else {
                    continue;
                };
                let first = (min_db / step).ceil() * step;
                for index in 0..=FRACTIONAL_INTERVALS_PER_SIDE {
                    let db = quantize_to_step(first + index as f64 * step, step);
                    if db > max_db + step * 1e-6 {
                        break;
                    }
                    // Integer candidates already exist and keep their established priorities.
                    if is_multiple_of_db(db, 1.0) {
                        continue;
                    }
                    let value = sign * 10.0_f64.powf(db / 20.0);
                    if let Some(screen_y) = sample_value_to_screen_y(value, range, screen, scale)
                        && screen.contains_y(screen_y)
                    {
                        self.finest_label_step_db = self.finest_label_step_db.min(step);
                        self.label_candidates.push(DbTick {
                            db,
                            sample_value: value,
                            screen_y,
                            tick_type: TickType::Small,
                        });
                    }
                }
            }
        }
    }
}

fn fractional_band_count(height: f32) -> usize {
    (height / FRACTIONAL_BAND_HEIGHT)
        .ceil()
        .clamp(1.0, MAX_FRACTIONAL_BANDS as f32) as usize
}

fn pick_step(candidates: &[DbTick], nr_pixels_per_tick: f32) -> f64 {
    // Check spacing per-sign because the positive- and negative-amplitude ticks for the same dB
    // value naturally converge near the zero crossing. Letting that cross-zero pair veto a
    // cadence would always demote us to the coarsest ladder entry on linear-amp axes.
    for &step in LADDER_DB {
        let pos_ok = same_sign_spacing_ok(candidates, step, nr_pixels_per_tick, |sv| sv >= 0.0);
        let neg_ok = same_sign_spacing_ok(candidates, step, nr_pixels_per_tick, |sv| sv <= 0.0);
        if pos_ok && neg_ok {
            return step;
        }
    }
    *LADDER_DB.last().unwrap()
}

fn same_sign_spacing_ok(
    candidates: &[DbTick],
    step: f64,
    min_gap_px: f32,
    side: impl Fn(f64) -> bool,
) -> bool {
    let mut last_y: Option<f32> = None;
    for tick in candidates
        .iter()
        .filter(|t| side(t.sample_value) && is_multiple_of_db(t.db, step))
    {
        if let Some(prev) = last_y
            && (tick.screen_y - prev).abs() < min_gap_px
        {
            return false;
        }
        last_y = Some(tick.screen_y);
    }
    true
}

fn minor_step_for(label_step_db: f64) -> f64 {
    // Half the label step where it makes sense on the ladder, otherwise 1 dB.
    match label_step_db as i32 {
        48 => 12.0,
        24 => 6.0,
        12 => 3.0,
        6 => 1.0,
        3 => 1.0,
        2 => 1.0,
        _ => 1.0,
    }
}

fn mid_step_for(label_step_db: f64) -> Option<f64> {
    match label_step_db as i32 {
        48 => Some(24.0),
        24 => Some(12.0),
        12 => Some(6.0),
        6 => Some(3.0),
        _ => None,
    }
}

pub(crate) fn is_multiple_of_db(value: f64, step: f64) -> bool {
    if step <= 0.0 {
        return false;
    }
    let quotient = value / step;
    let tolerance = (quotient.abs() * f64::EPSILON * 16.0).max(1e-8);
    (quotient - quotient.round()).abs() < tolerance
}

fn quantize_to_step(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return value;
    }
    (value / step).round() * step
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::db;

    #[test]
    fn off_zero_skew_keeps_visible_integer_and_fractional_candidates() {
        let screen = rect::Rect::new(0.0, 20.0, 80.0, 370.0);
        for sign in [-1.0, 1.0] {
            for skew_factor in [1.0, 3.0, 5.0] {
                let scale = ValueDisplayScale { skew_factor };
                let a = sign * scale.sample_to_display(10.0_f64.powf(-7.0 / 20.0));
                let b = sign * scale.sample_to_display(10.0_f64.powf(-5.0 / 20.0));
                let range = sample::ValRange {
                    min: a.min(b),
                    max: a.max(b),
                };
                let mut lattice = DbLattice::default();
                lattice.compute_ticks(range, screen, 50.0, scale).unwrap();
                assert!(
                    lattice.label_candidates.iter().any(|tick| tick.db == -6.0),
                    "sign={sign}, skew={skew_factor}: missing -6 dB"
                );
                assert!(
                    lattice
                        .label_candidates
                        .iter()
                        .any(|tick| !is_multiple_of_db(tick.db, 1.0)),
                    "sign={sign}, skew={skew_factor}: missing fractional ticks"
                );
            }
        }
    }

    #[test]
    fn narrow_positive_and_negative_ranges_have_fractional_candidates() {
        let screen = rect::Rect::new(0.0, 0.0, 80.0, 350.0);
        for sign in [-1.0, 1.0] {
            for (low_db, high_db) in [
                (-7.0, -5.0),
                (-6.004, -5.996),
                (-6.0002, -5.9998),
                (-6.0000002, -5.9999998),
            ] {
                let a = sign * 10.0_f64.powf(low_db / 20.0);
                let b = sign * 10.0_f64.powf(high_db / 20.0);
                let range = sample::ValRange {
                    min: a.min(b),
                    max: a.max(b),
                };
                let mut lattice = DbLattice::default();
                lattice
                    .compute_ticks(range, screen, 50.0, ValueDisplayScale::default())
                    .unwrap();
                assert!(
                    lattice.label_candidates.iter().any(|tick| tick.db == -6.0),
                    "the integer anchor must survive narrow ranges: sign={sign}, db range={low_db}..{high_db}, ticks={:?}",
                    lattice.label_candidates
                );
                if high_db - low_db > 1e-6 {
                    assert!(
                        lattice
                            .label_candidates
                            .iter()
                            .any(|tick| !is_multiple_of_db(tick.db, 1.0))
                    );
                    assert!(lattice.finest_label_step_db < 1.0);
                }
                assert!(
                    lattice
                        .label_candidates
                        .iter()
                        .all(|tick| tick.sample_value >= range.min
                            && tick.sample_value <= range.max
                            && tick.sample_value.signum() == sign
                            && screen.contains_y(tick.screen_y))
                );
            }
        }
    }

    #[test]
    fn candidate_budget_scales_with_height_and_has_a_hard_cap() {
        for height in [8.0, 80.0, 350.0, 1000.0, 100000.0] {
            for skew_factor in [0.0, 1.0, 5.0] {
                let mut lattice = DbLattice::default();
                lattice
                    .compute_ticks(
                        full_scale(),
                        rect::Rect::new(0.0, 0.0, 80.0, height),
                        50.0,
                        ValueDisplayScale { skew_factor },
                    )
                    .unwrap();
                let integer_budget = ((MAX_DB - MIN_DB) / FINE_STEP_DB + 1.0) as usize * 2;
                let fractional_budget =
                    fractional_band_count(height) * 2 * (FRACTIONAL_INTERVALS_PER_SIDE + 1);
                assert!(lattice.label_candidates.len() <= integer_budget + fractional_budget);
                assert!(
                    lattice.label_candidates.len()
                        <= integer_budget
                            + MAX_FRACTIONAL_BANDS * 2 * (FRACTIONAL_INTERVALS_PER_SIDE + 1)
                );
                for side in [-1.0, 1.0] {
                    let mut values: Vec<_> = lattice
                        .label_candidates
                        .iter()
                        .filter(|tick| tick.sample_value.signum() == side)
                        .map(|tick| tick.db)
                        .collect();
                    values.sort_by(f64::total_cmp);
                    assert!(
                        values
                            .windows(2)
                            .all(|pair| (pair[0] - pair[1]).abs() >= 1e-9)
                    );
                }
            }
        }
    }

    fn full_scale() -> sample::ValRange<f64> {
        sample::ValRange {
            min: -1.0,
            max: 1.0,
        }
    }

    #[test]
    fn crowded_ticks_near_zero_keep_coarse_marks_but_retain_roomy_label_candidates() {
        let range = sample::ValRange {
            min: -0.75,
            max: 0.45,
        };
        let screen = rect::Rect::new(0.0, 0.0, 80.0, 350.0);
        let scale = ValueDisplayScale::default();
        let y = |value_db: f32| {
            sample_value_to_screen_y(-(db::db_to_gain(value_db) as f64), range, screen, scale)
                .unwrap()
        };
        assert!((y(-6.0) - y(-12.0)).abs() > 50.0);
        assert!((y(-24.0) - y(-48.0)).abs() < 50.0);
        let mut lattice = DbLattice::default();
        lattice.compute_ticks(range, screen, 50.0, scale).unwrap();
        assert_eq!(lattice.label_step_db, 48.0);
        assert_eq!(lattice.minor_step_db, 12.0);
        assert!(
            lattice
                .ticks
                .iter()
                .all(|tick| is_multiple_of_db(tick.db, 12.0))
        );
        assert!(
            !lattice
                .ticks
                .iter()
                .any(|tick| tick.db == -6.0 && tick.sample_value < 0.0)
        );
        assert!(
            lattice
                .label_candidates
                .iter()
                .any(|tick| tick.db == -6.0 && tick.sample_value < 0.0)
        );
    }

    #[test]
    fn full_scale_range_emits_zero_db_at_both_extremes() {
        let mut lattice = DbLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 80.0, 240.0);
        lattice
            .compute_ticks(
                full_scale(),
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        let zero_db_count = lattice.ticks.iter().filter(|t| t.db == 0.0).count();
        assert_eq!(
            zero_db_count, 2,
            "0 dB should appear at both +1.0 and -1.0 amplitudes"
        );
        assert!(
            lattice
                .ticks
                .iter()
                .all(|t| screen_rect.contains_y(t.screen_y))
        );
    }

    #[test]
    fn label_cadence_snaps_to_ladder() {
        let mut lattice = DbLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 80.0, 240.0);
        lattice
            .compute_ticks(
                full_scale(),
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        assert!(
            LADDER_DB.contains(&lattice.label_step_db),
            "label_step_db={} must be on the ladder {:?}",
            lattice.label_step_db,
            LADDER_DB
        );
    }

    #[test]
    fn same_sign_big_ticks_are_at_least_nr_pixels_per_tick_apart() {
        // The mirrored pair around the zero crossing naturally converges, so we only require
        // the cadence to be readable on each side of zero independently.
        let mut lattice = DbLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 80.0, 240.0);
        let nr_pixels_per_tick = 50.0_f32;
        lattice
            .compute_ticks(
                full_scale(),
                screen_rect,
                nr_pixels_per_tick,
                ValueDisplayScale::default(),
            )
            .unwrap();

        for side_filter in [|sv: f64| sv >= 0.0, |sv: f64| sv <= 0.0] {
            let mut ys: Vec<f32> = lattice
                .ticks
                .iter()
                .filter(|t| t.tick_type == TickType::Big && side_filter(t.sample_value))
                .map(|t| t.screen_y)
                .collect();
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for window in ys.windows(2) {
                assert!(
                    (window[1] - window[0]).abs() >= nr_pixels_per_tick - 0.5,
                    "Same-sign Big tick spacing {} < {}",
                    (window[1] - window[0]).abs(),
                    nr_pixels_per_tick
                );
            }
        }
    }

    #[test]
    fn ticks_stay_inside_screen_rect() {
        let mut lattice = DbLattice::default();
        let screen_rect = rect::Rect::new(10.0, 20.0, 90.0, 220.0);
        lattice
            .compute_ticks(
                full_scale(),
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        for tick in &lattice.ticks {
            assert!(screen_rect.contains_y(tick.screen_y));
        }
    }

    #[test]
    fn shifted_positive_only_range_emits_only_positive_ticks() {
        let mut lattice = DbLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 80.0, 200.0);
        lattice
            .compute_ticks(
                sample::ValRange { min: 0.2, max: 0.9 },
                screen_rect,
                30.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        assert!(!lattice.ticks.is_empty());
        assert!(lattice.ticks.iter().all(|t| t.sample_value > 0.0));
    }

    #[test]
    fn skew_lets_deeper_db_ticks_through_than_linear() {
        // At skew = 2.3 the central region is blown up, so far deeper dB values map to pixels
        // still well clear of the zero crossing. The previous linear cap clipped at ~-60 dB
        // for a 1000-px-tall rect; the skew-aware probe should let -60+ dB ticks through.
        let mut linear = DbLattice::default();
        let mut skewed = DbLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 80.0, 1000.0);
        linear
            .compute_ticks(
                full_scale(),
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();
        skewed
            .compute_ticks(
                full_scale(),
                screen_rect,
                50.0,
                ValueDisplayScale { skew_factor: 2.3 },
            )
            .unwrap();

        let deepest_linear = linear.ticks.iter().map(|t| t.db).fold(0.0_f64, f64::min);
        let deepest_skewed = skewed.ticks.iter().map(|t| t.db).fold(0.0_f64, f64::min);

        assert!(
            deepest_skewed < deepest_linear,
            "skew should reach deeper dB than linear: linear={deepest_linear} skewed={deepest_skewed}"
        );
        assert!(
            deepest_skewed <= -60.0,
            "expected skewed deepest dB <= -60, got {deepest_skewed}"
        );
    }

    #[test]
    fn skewed_axis_still_produces_in_bounds_ticks() {
        let mut lattice = DbLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 80.0, 240.0);
        lattice
            .compute_ticks(
                full_scale(),
                screen_rect,
                50.0,
                ValueDisplayScale { skew_factor: 1.0 },
            )
            .unwrap();

        assert!(!lattice.ticks.is_empty());
        for tick in &lattice.ticks {
            assert!(screen_rect.contains_y(tick.screen_y));
        }
    }
}
