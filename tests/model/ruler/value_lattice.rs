use wavalyze::{
    audio::sample,
    model::ruler::{TickType, ValueLattice},
    rect::Rect,
};

#[test]
fn value_lattice_full_scale_zero_is_major() {
    let mut lattice = ValueLattice::default();
    lattice
        .compute_ticks(
            sample::ValRange {
                min: -1.0,
                max: 1.0,
            },
            Rect::new(0.0, 0.0, 40.0, 220.0),
            50.0,
        )
        .unwrap();

    assert!(lattice
        .ticks
        .iter()
        .any(|tick| tick.sample_value == 0.0 && tick.tick_type == TickType::Big));
    assert_eq!(lattice.major_step, 0.5);
}

#[test]
fn value_lattice_zoomed_range_stays_visible() {
    let mut lattice = ValueLattice::default();
    lattice
        .compute_ticks(
            sample::ValRange {
                min: -0.2,
                max: 0.2,
            },
            Rect::new(0.0, 0.0, 40.0, 240.0),
            50.0,
        )
        .unwrap();

    assert!(lattice
        .ticks
        .iter()
        .all(|tick| tick.sample_value >= -0.2 && tick.sample_value <= 0.2));
    assert_eq!(lattice.major_step, 0.1);
}
