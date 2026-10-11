//! Shared channel-order and active-layout controls for files and diff inputs.

use crate::{
    audio::Layout,
    wav::channel_interpretation::{
        ChannelInterpretation, ChannelOrderPreset, InterpretationSource,
    },
};

pub fn source_label(source: InterpretationSource) -> &'static str {
    match source {
        InterpretationSource::Wavex => "WAVEX",
        InterpretationSource::AuroIdOrder { assumed: true } => "Auro ID order (assumed)",
        InterpretationSource::AuroIdOrder { assumed: false } => "Auro ID order",
        InterpretationSource::Unassigned => "Unassigned",
    }
}

pub fn layout_label(layout: &Layout) -> String {
    let names = layout.names();
    if names.is_empty() {
        format!("{layout:?}")
    } else {
        names.join(" / ")
    }
}

/// Draw only the order selector, for compact table cells with their own column heading.
pub fn order_selector(ui: &mut egui::Ui, preset: &mut ChannelOrderPreset) -> egui::Response {
    egui::ComboBox::from_id_salt("channel_order")
        .selected_text(preset_label(*preset))
        .truncate()
        .show_ui(ui, |ui| {
            for option in [
                ChannelOrderPreset::Automatic,
                ChannelOrderPreset::AuroIdOrder,
                ChannelOrderPreset::Unassigned,
            ] {
                ui.selectable_value(preset, option, preset_label(option));
            }
        })
        .response
        .on_hover_text("Automatic uses a valid WAVEX mask; absent or zero masks assume Auro ID order. Invalid nonzero masks stay unassigned.")
}

/// Edit an interpretation without doing disk work or changing audio; the caller commits it.
/// Call inside a per-file/input ID scope so each selector has independent popup state.
pub fn order_controls(
    ui: &mut egui::Ui,
    interpretation: &mut ChannelInterpretation,
    channel_mask: Option<u32>,
    total_nr_channels: usize,
) -> bool {
    let original = interpretation.clone();
    ui.horizontal(|ui| {
        ui.label("Channel order:");
        order_selector(ui, &mut interpretation.preset);
    });
    let resolved = interpretation.resolve_and_validate(channel_mask, total_nr_channels);
    ui.small(source_label(resolved.source));
    *interpretation != original
}

/// Draw the value cell of the active-layout metadata row.
pub fn layout_selector(
    ui: &mut egui::Ui,
    interpretation: &mut ChannelInterpretation,
    channel_mask: Option<u32>,
    total_nr_channels: usize,
) -> bool {
    let original = interpretation.clone();
    let resolved = interpretation.resolve_and_validate(channel_mask, total_nr_channels);
    if matches!(resolved.source, InterpretationSource::AuroIdOrder { .. }) {
        ui.add_enabled_ui(!resolved.candidate_layouts.is_empty(), |ui| {
            egui::ComboBox::from_id_salt("active_layout")
                .selected_text(interpretation.active_layout.as_ref().map(layout_label).unwrap_or_else(|| "Unknown".into()))
                .truncate()
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut interpretation.active_layout, None, "Unknown");
                    for layout in &resolved.candidate_layouts {
                        ui.selectable_value(&mut interpretation.active_layout, Some(layout.clone()), layout_label(layout));
                    }
                })
                .response
                .on_hover_text(if resolved.candidate_layouts.is_empty() {
                    "No compatible named layouts for this padded channel count."
                } else {
                    "Compatible layouts for this padded channel count. Choosing one marks expected padding without checking silence or changing audio."
                });
        });
    } else if let Some(layout) = resolved.active_layout {
        ui.label(format!("{} (WAVEX)", layout_label(&layout)));
    } else {
        ui.label("Unknown");
    }
    *interpretation != original
}

fn preset_label(preset: ChannelOrderPreset) -> &'static str {
    match preset {
        ChannelOrderPreset::Automatic => "Automatic",
        ChannelOrderPreset::AuroIdOrder => "Auro ID order",
        ChannelOrderPreset::Unassigned => "Unassigned",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ui(
        ui: &mut egui::Ui,
        interpretation: &mut ChannelInterpretation,
        mask: Option<u32>,
        count: usize,
    ) -> bool {
        let mut changed = order_controls(ui, interpretation, mask, count);
        egui::Grid::new("metadata_test").show(ui, |ui| {
            ui.label("active layout");
            changed |= layout_selector(ui, interpretation, mask, count);
            ui.end_row();
        });
        changed
    }

    fn label_center(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, label))
            .unwrap_or_else(|| panic!("missing label {label:?}"))
    }

    #[test]
    fn selectors_choose_mono_then_clear_it_when_roles_are_unassigned() {
        let ctx = egui::Context::default();
        let mut interpretation = ChannelInterpretation::default();
        let mut frame = |events| {
            let mut changed = false;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 500.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        changed |= test_ui(ui, &mut interpretation, None, 3);
                    });
                },
            );
            (output, changed)
        };
        let click = |position| {
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ]
        };
        let (output, changed) = frame(vec![]);
        assert!(!changed);
        let unknown = label_center(&output, "Unknown");
        let _ = frame(click(unknown));
        let (output, _) = frame(vec![]);
        let mono = label_center(&output, "1.0 / TestMix1.0");
        let (_, changed) = frame(click(mono));
        assert!(changed);
        let (output, _) = frame(vec![]);
        label_center(&output, "1.0 / TestMix1.0");
        let automatic = label_center(&output, "Automatic");
        let _ = frame(click(automatic));
        let (output, _) = frame(vec![]);
        let unassigned = label_center(&output, "Unassigned");
        let (_, changed) = frame(click(unassigned));
        assert!(changed);
        assert_eq!(interpretation.preset, ChannelOrderPreset::Unassigned);
        assert_eq!(interpretation.active_layout, None);
    }

    #[test]
    fn wavex_layout_is_shown_without_an_active_layout_selector() {
        let ctx = egui::Context::default();
        let mut interpretation = ChannelInterpretation::default();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(!test_ui(ui, &mut interpretation, Some(0x60f), 6));
            });
        });
        label_center(&output, "5.1 (WAVEX)");
        assert_eq!(interpretation.active_layout, None);
    }
}
