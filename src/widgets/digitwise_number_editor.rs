use egui::text::{CCursor, CCursorRange};
use egui::{self, Response, TextEdit, TextStyle, Ui, WidgetText};

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
}

struct RenderedDigit {
    id: egui::Id,
    response: Response,
    state: egui::text_edit::TextEditState,
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
        let desired_width = self
            .digit_width
            .unwrap_or_else(|| default_digit_width(ui) + ui.spacing().button_padding.x * 2.0);

        let mut changed = false;
        let mut action = None;
        let mut select_digit = None;
        let mut rendered_digits = Vec::with_capacity(digits);

        let inner = ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 2.0;

            for (digit_index, digit_char) in digit_chars.iter().copied().enumerate() {
                let digit_id = self.id_source.with(("digit", digit_index));
                let mut digit_text = digit_char.to_string();
                let output = TextEdit::singleline(&mut digit_text)
                    .id(digit_id)
                    .font(TextStyle::Monospace)
                    .desired_width(desired_width)
                    .char_limit(1)
                    .horizontal_align(egui::Align::Center)
                    .clip_text(false)
                    .show(ui);

                if output.response.clicked() {
                    state.selected_digit = digit_index;
                    action = Some(DigitwiseNumberEditorAction::FocusDigit);
                    select_digit = Some(digit_index);
                }

                if output.response.has_focus() {
                    state.selected_digit = digit_index;
                }

                if let Some(new_digit) = parse_single_ascii_digit(&digit_text) {
                    if output.response.changed() {
                        state.selected_digit = digit_index;
                        let current_digit = digit_char.to_digit(10).expect("digit char") as u8;
                        if new_digit != current_digit
                            && apply_replace_digit(
                                self.value,
                                digits,
                                digit_index,
                                new_digit,
                                clamped_max,
                            )
                        {
                            changed = true;
                            action = Some(DigitwiseNumberEditorAction::ReplaceDigit);
                            select_digit = Some((digit_index + 1).min(digits - 1));
                        } else {
                            select_digit = Some(digit_index);
                        }
                    }
                } else if output.response.changed() {
                    state.selected_digit = digit_index;
                    select_digit = Some(digit_index);
                }

                rendered_digits.push(RenderedDigit {
                    id: digit_id,
                    response: output.response,
                    state: output.state,
                });
            }
        });

        let mut response = inner.response;
        for digit in &rendered_digits {
            response = response.union(digit.response.clone());
        }

        if let Some(focused_digit) = rendered_digits
            .iter()
            .position(|digit| digit.response.has_focus())
        {
            state.selected_digit = focused_digit;

            if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft)) && focused_digit > 0 {
                state.selected_digit = focused_digit - 1;
                action = Some(DigitwiseNumberEditorAction::MoveLeft);
                select_digit = Some(focused_digit - 1);
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowRight))
                && focused_digit + 1 < digits
            {
                state.selected_digit = focused_digit + 1;
                action = Some(DigitwiseNumberEditorAction::MoveRight);
                select_digit = Some(focused_digit + 1);
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowUp))
                && apply_step_at_digit(self.value, digits, focused_digit, 1, clamped_max)
            {
                changed = true;
                action = Some(DigitwiseNumberEditorAction::IncrementPlace);
                select_digit = Some(focused_digit);
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown))
                && apply_step_at_digit(self.value, digits, focused_digit, -1, clamped_max)
            {
                changed = true;
                action = Some(DigitwiseNumberEditorAction::DecrementPlace);
                select_digit = Some(focused_digit);
            }
        }

        if let Some(digit_index) = select_digit {
            select_digit_text(ui, &mut rendered_digits[digit_index]);
            state.selected_digit = digit_index;
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

fn select_digit_text(ui: &Ui, digit: &mut RenderedDigit) {
    ui.memory_mut(|memory| memory.request_focus(digit.id));
    digit
        .state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(1))));
    digit.state.clone().store(ui.ctx(), digit.id);
}

fn default_digit_width(ui: &Ui) -> f32 {
    let galley = WidgetText::from("0").into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Monospace,
    );
    galley.size().x
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

fn parse_single_ascii_digit(text: &str) -> Option<u8> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    ch.to_digit(10).map(|digit| digit as u8)
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
    fn parse_single_ascii_digit_rejects_invalid_text() {
        assert_eq!(parse_single_ascii_digit("7"), Some(7));
        assert_eq!(parse_single_ascii_digit(""), None);
        assert_eq!(parse_single_ascii_digit("12"), None);
        assert_eq!(parse_single_ascii_digit("x"), None);
    }
}
