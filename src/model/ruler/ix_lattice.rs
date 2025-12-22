use crate::{
    audio::sample,
    math::round::round_up_to_power_of_10,
    model::ruler::{ceil_to_multiple, floor_to_multiple, sample_ix_to_screen_x},
    rect,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickType {
    /// Not all ticks are labeled, labeled ticks are always a multiple of 10
    Labeled,
    /// Halfway between two labeled ticks
    Mid,
    /// All other 1/10 inbetween ticks
    Small,
}

/// A tick (lattice/grid line) on the ruler
#[derive(Debug, Clone)]
pub struct Tick {
    /// The sample ix of this tick, can be negative
    pub sample_ix: i64,

    /// Pixel position of this tick
    pub screen_x: f32,

    /// The type of this tick
    pub tick_type: TickType,
}

#[derive(Debug, Clone, Default)]
pub struct IxLattice {
    pub ticks: Vec<Tick>,
}

impl IxLattice {
    pub fn compute_ticks(
        &mut self,
        sample_ix_range: sample::FracIxRange,
        screen_rect: rect::Rect,
        nr_pixels_per_tick: f32,
    ) -> anyhow::Result<()> {
        // tracing::trace!("compute_ticks {:?} {:?}", sample_ix_range, screen_rect.x_range_inc());
        let max_nr_ticks: f32 = screen_rect.width() / nr_pixels_per_tick;
        let sample_width = sample_ix_range.len();
        if sample_width == 0.0 {
            anyhow::bail!("sample width is zero, cannot draw grid");
        }

        self.ticks.clear();

        let min_nr_samples_per_label_tick: f64 = sample_width / max_nr_ticks as f64;
        let nr_samples_per_label_tick: u64 = round_up_to_power_of_10(min_nr_samples_per_label_tick) as u64;
        // When very zoomed in, this could be smaller than 1 (i.e. 0), but we want to have at least
        // 1 tick per sample
        let nr_samples_per_label_tick: u64 = nr_samples_per_label_tick.max(1);
        let nr_samples_per_mid_tick: u64 = (nr_samples_per_label_tick / 2).max(1);
        let nr_samples_per_small_tick: u64 = (nr_samples_per_label_tick / 10).max(1);

        let multiple = nr_samples_per_small_tick as i64;
        let start_sample_ix = ceil_to_multiple(sample_ix_range.start.ceil() as i64, multiple);
        let end_sample_ix = floor_to_multiple(sample_ix_range.end.floor() as i64, multiple);
        for cur_sample_ix in (start_sample_ix..=end_sample_ix).step_by(nr_samples_per_small_tick as usize) {
            let Some(screen_x) = sample_ix_to_screen_x(cur_sample_ix as f64, sample_ix_range, screen_rect) else {
                continue;
            };
            let tick_type = if cur_sample_ix % nr_samples_per_label_tick as i64 == 0 {
                TickType::Labeled
            } else if cur_sample_ix % nr_samples_per_mid_tick as i64 == 0 {
                TickType::Mid
            } else {
                TickType::Small
            };
            self.ticks.push(Tick {
                sample_ix: cur_sample_ix,
                screen_x,
                tick_type,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticks() {
        let mut lattice = IxLattice::default();
        let sample_ix_range = sample::FracIxRange { start: -120.0, end: 100.0 };
        let screen_rect = rect::Rect::new(0.0, 0.0, 1000.0, 0.0);
        lattice.compute_ticks(sample_ix_range, screen_rect, 100.0).unwrap();
        dbg!(lattice);
    }
}
