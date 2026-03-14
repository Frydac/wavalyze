use egui::{self, Align2, Color32, EventFilter, FontId, Response, Sense, Stroke, Ui, WidgetText};

const MAX_U64_DIGITS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitwiseNumberEditorAction {
    FocusDigit,
    MoveLeft,
    MoveRight,
    ReplaceDigit,
    IncrementPlace,
    DecrementPlace,
}

#[derive(Debug)]
pub struct DigitwiseNumberEditorOutput {
    pub response: Response,
    pub changed: bool,
    pub selected_digit: usize,
    pub action: Option<DigitwiseNumberEditorAction>,
}

#[derive(Debug)]
pub struct DigitwiseNumberEditor<'a> {
    id_source: egui::Id,
    value: &'a mut u64,
    digits: usize,
    max: u64,
    digit_width: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct EditorState {
    selected_digit: usize,
    has_focus: bool,
}

struct RenderedDigit {
    id: egui::Id,
    response: Response,
}

impl<'a> DigitwiseNumberEditor<'a> {
    pub fn new(id_source: impl std::hash::Hash, value: &'a mut u64) -> Self {
        Self {
            id_source: egui::Id::new(id_source),
            value,
            digits: 1,
            max: u64::MAX,
            digit_width: None,
        }
    }

    pub fn digits(mut self, digits: usize) -> Self {
        self.digits = digits;
        self
    }

    pub fn max(mut self, max: u64) -> Self {
        self.max = max;
        self
    }

    pub fn digit_width(mut self, digit_width: f32) -> Self {
        self.digit_width = Some(digit_width);
        self
    }

    pub fn show(self, ui: &mut Ui) -> DigitwiseNumberEditorOutput {
        let digits = normalize_digits(self.digits);
        let clamped_max = self.max.min(max_value_for_digits(digits));
        *self.value = (*self.value).min(clamped_max);

        let mut state = load_state(ui.ctx(), self.id_source);
        state.selected_digit = state.selected_digit.min(digits - 1);

        let displayed_value = format_value(*self.value, digits);
        let digit_chars: Vec<char> = displayed_value.chars().collect();
        let digit_size = digit_size(ui, self.digit_width);
        let font_id = FontId::monospace(ui.style().text_styles[&egui::TextStyle::Monospace].size);

        let mut changed = false;
        let mut action = None;
        let mut focus_digit = None;
        let mut rendered_digits = Vec::with_capacity(digits);

        let inner = ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 1.0;

            for (digit_index, digit_char) in digit_chars.iter().copied().enumerate() {
                let digit_id = self.id_source.with(("digit", digit_index));
                let (rect, _) = ui.allocate_exact_size(digit_size, Sense::hover());
                let response = ui.interact(rect, digit_id, Sense::click());

                if response.clicked() {
                    state.selected_digit = digit_index;
                    state.has_focus = true;
                    action = Some(DigitwiseNumberEditorAction::FocusDigit);
                    focus_digit = Some(digit_index);
                }

                let has_focus = ui.memory(|memory| memory.has_focus(digit_id))
                    || (state.has_focus && state.selected_digit == digit_index);
                if has_focus {
                    state.selected_digit = digit_index;
                }

                paint_digit(ui, rect, digit_char, &font_id, has_focus);

                rendered_digits.push(RenderedDigit {
                    id: digit_id,
                    response,
                });

                if has_group_separator(digits, digit_index) {
                    paint_separator(ui, &font_id);
                }
            }
        });

        let mut response = inner.response;
        for digit in &rendered_digits {
            response = response.union(digit.response.clone());
        }

        let focused_digit = rendered_digits
            .iter()
            .position(|digit| ui.memory(|memory| memory.has_focus(digit.id)))
            .or_else(|| focus_digit.map(|_| state.selected_digit));

        if let Some(focused_digit) = focused_digit {
            state.selected_digit = focused_digit;
            state.has_focus = true;

            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    rendered_digits[focused_digit].id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });

            let left_presses = ui.input(|i| i.num_presses(egui::Key::ArrowLeft));
            let right_presses = ui.input(|i| i.num_presses(egui::Key::ArrowRight));
            let up_presses = ui.input(|i| i.num_presses(egui::Key::ArrowUp));
            let down_presses = ui.input(|i| i.num_presses(egui::Key::ArrowDown));

            if left_presses > 0 && focused_digit > 0 {
                state.selected_digit = focused_digit - 1;
                action = Some(DigitwiseNumberEditorAction::MoveLeft);
                focus_digit = Some(focused_digit - 1);
            } else if right_presses > 0 && focused_digit + 1 < digits {
                state.selected_digit = focused_digit + 1;
                action = Some(DigitwiseNumberEditorAction::MoveRight);
                focus_digit = Some(focused_digit + 1);
            } else if up_presses > 0 {
                let mut any_change = false;
                for _ in 0..up_presses {
                    if apply_step_at_digit(self.value, digits, focused_digit, 1, clamped_max) {
                        any_change = true;
                    }
                }
                if any_change {
                    changed = true;
                    action = Some(DigitwiseNumberEditorAction::IncrementPlace);
                }
                focus_digit = Some(focused_digit);
            } else if down_presses > 0 {
                let mut any_change = false;
                for _ in 0..down_presses {
                    if apply_step_at_digit(self.value, digits, focused_digit, -1, clamped_max) {
                        any_change = true;
                    }
                }
                if any_change {
                    changed = true;
                    action = Some(DigitwiseNumberEditorAction::DecrementPlace);
                }
                focus_digit = Some(focused_digit);
            } else if let Some(input) = typed_digit_input(ui) {
                let current_digit =
                    digit_chars[focused_digit].to_digit(10).expect("digit char") as u8;

                if let Some(new_digit) = input {
                    if new_digit != current_digit
                        && apply_replace_digit(
                            self.value,
                            digits,
                            focused_digit,
                            new_digit,
                            clamped_max,
                        )
                    {
                        changed = true;
                        action = Some(DigitwiseNumberEditorAction::ReplaceDigit);
                        let next_digit = (focused_digit + 1).min(digits - 1);
                        state.selected_digit = next_digit;
                        focus_digit = Some(next_digit);
                    } else {
                        focus_digit = Some(focused_digit);
                    }
                } else {
                    focus_digit = Some(focused_digit);
                }
            }
        } else {
            state.has_focus = false;
        }

        if let Some(digit_index) = focus_digit {
            ui.memory_mut(|memory| memory.request_focus(rendered_digits[digit_index].id));
            state.selected_digit = digit_index;
            state.has_focus = true;
        }

        store_state(ui.ctx(), self.id_source, state);

        DigitwiseNumberEditorOutput {
            response,
            changed,
            selected_digit: state.selected_digit,
            action,
        }
    }
}

fn load_state(ctx: &egui::Context, id: egui::Id) -> EditorState {
    ctx.data_mut(|data| data.get_temp(id)).unwrap_or_default()
}

fn store_state(ctx: &egui::Context, id: egui::Id, state: EditorState) {
    ctx.data_mut(|data| data.insert_temp(id, state));
}

fn digit_size(ui: &Ui, digit_width: Option<f32>) -> egui::Vec2 {
    let glyph_size = glyph_size(ui);
    egui::vec2(
        digit_width.unwrap_or(glyph_size.x + 4.0),
        glyph_size.y + 4.0,
    )
}

fn glyph_size(ui: &Ui) -> egui::Vec2 {
    let galley = WidgetText::from("0").into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Monospace,
    );
    galley.size()
}

fn paint_digit(ui: &Ui, rect: egui::Rect, digit_char: char, font_id: &FontId, has_focus: bool) {
    let base_bg = ui.visuals().extreme_bg_color;
    let bg_fill = if has_focus {
        base_bg.linear_multiply(5.0)
    } else {
        base_bg.linear_multiply(0.9)
    };

    ui.painter()
        .rect(rect, 1.5, bg_fill, Stroke::new(0.0, Color32::TRANSPARENT));
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        digit_char,
        font_id.clone(),
        ui.visuals().text_color(),
    );
}

fn paint_separator(ui: &mut Ui, font_id: &FontId) {
    let glyph_size = glyph_size(ui);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(glyph_size.x, glyph_size.y + 4.0), Sense::hover());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ",",
        font_id.clone(),
        ui.visuals().weak_text_color(),
    );
}

fn typed_digit_input(ui: &Ui) -> Option<Option<u8>> {
    ui.input(|input| {
        if input.modifiers.command || input.modifiers.ctrl || input.modifiers.alt {
            return None;
        }

        for event in &input.events {
            if let egui::Event::Text(text) = event {
                let mut chars = text.chars();
                let ch = chars.next()?;
                if chars.next().is_some() {
                    return Some(None);
                }
                return Some(ch.to_digit(10).map(|digit| digit as u8));
            }
        }

        None
    })
}

fn normalize_digits(digits: usize) -> usize {
    digits.clamp(1, MAX_U64_DIGITS)
}

fn max_value_for_digits(digits: usize) -> u64 {
    if digits >= MAX_U64_DIGITS {
        return u64::MAX;
    }

    pow10(digits as u32).unwrap_or(u64::MAX).saturating_sub(1)
}

fn format_value(value: u64, digits: usize) -> String {
    format!("{value:0digits$}")
}

fn has_group_separator(digits: usize, digit_index: usize) -> bool {
    let remaining_digits = digits.saturating_sub(digit_index + 1);
    remaining_digits > 0 && remaining_digits.is_multiple_of(3)
}

fn apply_replace_digit(
    value: &mut u64,
    digits: usize,
    digit_index: usize,
    new_digit: u8,
    max: u64,
) -> bool {
    let next = replace_digit(*value, digits, digit_index, new_digit);
    if next > max || next == *value {
        return false;
    }
    *value = next;
    true
}

fn replace_digit(value: u64, digits: usize, digit_index: usize, new_digit: u8) -> u64 {
    let place = digits.saturating_sub(digit_index + 1) as u32;
    let factor = pow10(place).unwrap_or(u64::MAX);
    let current_digit = ((value / factor) % 10) as u8;
    let removed = value.saturating_sub(current_digit as u64 * factor);
    removed.saturating_add(new_digit as u64 * factor)
}

fn apply_step_at_digit(
    value: &mut u64,
    digits: usize,
    digit_index: usize,
    delta_sign: i8,
    max: u64,
) -> bool {
    let step = digit_step(digits, digit_index);
    let next = match delta_sign {
        1 => (*value).saturating_add(step).min(max),
        -1 => (*value).saturating_sub(step),
        _ => *value,
    };

    if next == *value {
        return false;
    }

    *value = next;
    true
}

fn digit_step(digits: usize, digit_index: usize) -> u64 {
    let place = digits.saturating_sub(digit_index + 1) as u32;
    pow10(place).unwrap_or(u64::MAX)
}

fn pow10(power: u32) -> Option<u64> {
    10_u64.checked_pow(power)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_digit_updates_correct_place_value() {
        assert_eq!(replace_digit(12_345, 5, 0, 9), 92_345);
        assert_eq!(replace_digit(12_345, 5, 2, 0), 12_045);
        assert_eq!(replace_digit(12_345, 5, 4, 9), 12_349);
    }

    #[test]
    fn step_size_matches_selected_digit() {
        assert_eq!(digit_step(6, 0), 100_000);
        assert_eq!(digit_step(6, 2), 1_000);
        assert_eq!(digit_step(6, 5), 1);
    }

    #[test]
    fn increment_and_decrement_clamp_to_bounds() {
        let mut value = 98;
        assert!(apply_step_at_digit(&mut value, 2, 0, 1, 99));
        assert_eq!(value, 99);
        assert!(!apply_step_at_digit(&mut value, 2, 0, 1, 99));
        assert_eq!(value, 99);

        value = 3;
        assert!(apply_step_at_digit(&mut value, 2, 1, -1, 99));
        assert_eq!(value, 2);
        value = 0;
        assert!(!apply_step_at_digit(&mut value, 2, 1, -1, 99));
        assert_eq!(value, 0);
    }

    #[test]
    fn replace_digit_result_clamps_to_max() {
        let mut value = 59;
        assert!(!apply_replace_digit(&mut value, 2, 0, 9, 75));
        assert_eq!(value, 59);
    }

    #[test]
    fn format_value_keeps_leading_zeroes() {
        assert_eq!(format_value(42, 6), "000042");
    }

    #[test]
    fn max_value_matches_digit_count() {
        assert_eq!(max_value_for_digits(1), 9);
        assert_eq!(max_value_for_digits(4), 9_999);
        assert_eq!(max_value_for_digits(MAX_U64_DIGITS), u64::MAX);
    }

    #[test]
    fn group_separator_every_three_digits() {
        assert!(has_group_separator(6, 2));
        assert!(has_group_separator(6, 5 - 3));
        assert!(!has_group_separator(6, 4));
    }
}
