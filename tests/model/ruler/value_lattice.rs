use wavalyze::{
    audio::sample,
    model::ruler::{TickType, ValueDisplayScale, ValueLattice},
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
            ValueDisplayScale::default(),
        )
        .unwrap();

    assert!(
        lattice
            .ticks
            .iter()
            .any(|tick| tick.sample_value == 0.0 && tick.tick_type == TickType::Big)
    );
    assert_eq!(lattice.major_step, 0.5);
    assert_eq!(lattice.mid_step, None);
    assert_eq!(lattice.minor_step, 0.1);
    assert_eq!(lattice.label_step, 0.5);
    assert_eq!(
        lattice
            .ticks
            .iter()
            .filter(|tick| {
                tick.tick_type == TickType::Small
                    && tick.sample_value > -1.0
                    && tick.sample_value < -0.5
            })
            .count(),
        4
    );
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
            ValueDisplayScale::default(),
        )
        .unwrap();

    assert!(
        lattice
            .ticks
            .iter()
            .all(|tick| tick.sample_value >= -0.2 && tick.sample_value <= 0.2)
    );
    assert_eq!(lattice.major_step, 0.1);
    assert_eq!(lattice.mid_step, Some(0.05));
    assert_eq!(lattice.minor_step, 0.01);
    assert_eq!(lattice.label_step, 0.1);
    assert!(
        lattice
            .ticks
            .iter()
            .any(|tick| tick.sample_value == -0.19 && tick.tick_type == TickType::Small)
    );
    assert!(
        lattice
            .ticks
            .iter()
            .any(|tick| tick.sample_value == -0.15 && tick.tick_type == TickType::Mid)
    );
    assert_eq!(
        lattice
            .ticks
            .iter()
            .filter(|tick| {
                tick.tick_type == TickType::Small
                    && tick.sample_value > -0.2
                    && tick.sample_value < -0.1
            })
            .count(),
        8
    );
}

#[test]
fn value_lattice_can_label_mid_ticks() {
    let mut lattice = ValueLattice::default();
    lattice
        .compute_ticks(
            sample::ValRange {
                min: -0.12,
                max: 0.12,
            },
            Rect::new(0.0, 0.0, 40.0, 240.0),
            50.0,
            ValueDisplayScale::default(),
        )
        .unwrap();

    assert_eq!(lattice.major_step, 0.1);
    assert_eq!(lattice.mid_step, Some(0.05));
    assert_eq!(lattice.minor_step, 0.01);
    assert_eq!(lattice.label_step, 0.05);
    assert!(
        lattice
            .ticks
            .iter()
            .any(|tick| tick.sample_value == 0.05 && tick.tick_type == TickType::Mid)
    );
}
