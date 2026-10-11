use crate::{
    audio::sample::ValRange,
    model::ruler::{DbLattice, ValueDisplayScale},
};

fn ranges() -> [ValRange<f64>; 5] {
    [
        ValRange {
            min: -1.0,
            max: 1.0,
        },
        ValRange {
            min: -0.75,
            max: 0.45,
        },
        ValRange {
            min: -0.01,
            max: 0.001,
        },
        ValRange {
            min: 0.45,
            max: 0.55,
        },
        ValRange {
            min: -0.5015,
            max: -0.5010,
        },
    ]
}

#[test]
fn labels_stay_inside_and_separated_across_heights_ranges_and_pixel_densities() {
    for pixels_per_point in [1.0, 1.5, 2.0] {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(pixels_per_point);
        for height in [24.0, 80.0, 350.0, 1000.0] {
            let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(80.0, height));
            for range in ranges() {
                // Include both zero-crossing and off-zero display ranges.
                for skew_factor in [0.0, 1.0, 5.0] {
                    let mut lattice = DbLattice::default();
                    lattice
                        .compute_ticks(
                            range,
                            rect.into(),
                            super::NR_PIXELS_PER_DB_TICK,
                            ValueDisplayScale { skew_factor },
                        )
                        .unwrap();
                    let mut occupied = Vec::new();
                    let _ = ctx.run(egui::RawInput::default(), |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            super::draw_lattice_labels(ui, rect, &lattice, &mut occupied);
                        });
                    });
                    for (index, label) in occupied.iter().enumerate() {
                        assert!(rect.contains_rect(*label));
                        assert!(
                            occupied[..index]
                                .iter()
                                .all(|other| !other.expand(2.0).intersects(label.expand(2.0)))
                        );
                    }
                    if height >= 80.0 && skew_factor == 0.0 {
                        assert!(
                            !occupied.is_empty(),
                            "range={range:?}, height={height}, pixels_per_point={pixels_per_point}"
                        );
                    }
                }
            }
        }
    }
}

/// Manual CPU measurement, excluding waveform rendering, tessellation and GPU work.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "manual performance measurement; run with --release --ignored --nocapture"]
fn measure_many_db_rulers() {
    use std::time::{Duration, Instant};
    const TRACKS: usize = 24;
    const FRAMES: usize = 220;
    let ctx = egui::Context::default();
    let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(80.0, 350.0));
    for changing in [false, true] {
        let mut generation = Vec::new();
        let mut placement = Vec::new();
        let mut max_candidates = 0;
        let mut lattices = vec![DbLattice::default(); TRACKS];
        for frame in 0..FRAMES {
            let started = Instant::now();
            for (index, lattice) in lattices.iter_mut().enumerate() {
                let mut range = ranges()[index % ranges().len()];
                if changing {
                    let zoom = 1.0 + (frame as f64 * 0.03).sin() * 0.1;
                    range.min *= zoom;
                    range.max *= zoom;
                }
                let skew_factor = if range.min < 0.0 && range.max > 0.0 {
                    (index % 3) as f32
                } else {
                    0.0
                };
                lattice
                    .compute_ticks(
                        range,
                        rect.into(),
                        super::NR_PIXELS_PER_DB_TICK,
                        ValueDisplayScale { skew_factor },
                    )
                    .unwrap();
                max_candidates = max_candidates.max(lattice.label_candidates.len());
            }
            let generated = started.elapsed();
            let mut placed = Duration::ZERO;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let started = Instant::now();
                    for lattice in &lattices {
                        super::draw_lattice_labels(ui, rect, lattice, &mut Vec::new());
                    }
                    placed = started.elapsed();
                });
            });
            if frame >= 20 {
                generation.push(generated.as_secs_f64() * 1000.0);
                placement.push(placed.as_secs_f64() * 1000.0);
            }
        }
        generation.sort_by(f64::total_cmp);
        placement.sort_by(f64::total_cmp);
        println!(
            "24 rulers, changing={changing}, max candidates/ruler={max_candidates}: generation median/p95 {:.3}/{:.3} ms, placement median/p95 {:.3}/{:.3} ms",
            generation[100], generation[190], placement[100], placement[190]
        );
    }
}
