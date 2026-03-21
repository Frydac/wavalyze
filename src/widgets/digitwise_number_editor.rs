use egui::{self, Align2, Color32, EventFilter, FontId, Response, Sense, Stroke, Ui, WidgetText};

const MAX_U64_DIGITS: usize = 20;
const DRAG_START_THRESHOLD_PX: f32 = 4.0;
const DRAG_STEP_PX: f32 = 12.0;
const DIGITWISE_EDITOR_IDS_DATA_KEY: &str = "digitwise_number_editor_ids";
const GROUP_SEPARATOR_WIDTH_FACTOR: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitwiseNumberEditorAction {
    FocusDigit,
    MoveLeft,
    MoveRight,
    ReplaceDigit,
    IncrementPlace,
    DecrementPlace,
    DragAdjustPlace,
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
    dim_leading_zeroes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct EditorState {
    selected_digit: usize,
    has_saved_selection: bool,
    has_focus: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DragState {
    digit_index: usize,
    press_y: f32,
    applied_steps: i32,
    crossed_threshold: bool,
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
            dim_leading_zeroes: false,
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

    pub fn dim_leading_zeroes(mut self, dim_leading_zeroes: bool) -> Self {
        self.dim_leading_zeroes = dim_leading_zeroes;
        self
    }

    pub fn show(self, ui: &mut Ui) -> DigitwiseNumberEditorOutput {
        let digits = normalize_digits(self.digits);
        let clamped_max = self.max.min(max_value_for_digits(digits));
        *self.value = (*self.value).min(clamped_max);

        let editor_id = self.id_source.with("editor");
        let mut state = load_state(ui.ctx(), self.id_source);
        state.selected_digit = state.selected_digit.min(digits - 1);
        let drag_state_id = self.id_source.with("drag_state");
        let mut drag_state = load_drag_state(ui.ctx(), drag_state_id);

        let displayed_value = format_value(*self.value, digits);
        let digit_chars: Vec<char> = displayed_value.chars().collect();
        let digit_size = digit_size(ui, self.digit_width);
        let font_id = FontId::monospace(ui.style().text_styles[&egui::TextStyle::Monospace].size);

        let mut changed = false;
        let mut action = None;
        let mut request_editor_focus = false;
        let mut rendered_digits = Vec::with_capacity(digits);

        let inner = ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 1.0;

            for (digit_index, digit_char) in digit_chars.iter().copied().enumerate() {
                let digit_id = self.id_source.with(("digit", digit_index));
                let (rect, _) = ui.allocate_exact_size(digit_size, Sense::hover());
                let response = ui.interact(rect, digit_id, digit_interaction_sense());

                if response.drag_started()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                {
                    drag_state = Some(DragState {
                        digit_index,
                        press_y: pointer_pos.y,
                        applied_steps: 0,
                        crossed_threshold: false,
                    });
                }

                if response.clicked() {
                    state.selected_digit = digit_index;
                    state.has_saved_selection = true;
                    state.has_focus = true;
                    action = Some(DigitwiseNumberEditorAction::FocusDigit);
                    request_editor_focus = true;
                }

                let has_focus = ui.memory(|memory| memory.has_focus(editor_id))
                    && state.selected_digit == digit_index;

                let is_leading_zero =
                    self.dim_leading_zeroes && is_leading_zero_digit(&digit_chars, digit_index);
                paint_digit(ui, rect, digit_char, &font_id, has_focus, is_leading_zero);

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
        response = response.union(ui.interact(
            response.rect,
            editor_id,
            Sense::focusable_noninteractive(),
        ));
        register_digit_ids(
            ui.ctx(),
            std::iter::once(editor_id).chain(rendered_digits.iter().map(|digit| digit.id)),
        );

        let mut drag_focus_digit = None;
        if let Some(mut active_drag) = drag_state {
            if ui.input(|i| i.pointer.primary_down()) {
                if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                    let total_steps = drag_steps_from_pointer(active_drag.press_y, pointer_pos.y);
                    if !active_drag.crossed_threshold && total_steps != 0 {
                        active_drag.crossed_threshold = true;
                    }

                    if active_drag.crossed_threshold {
                        let step_delta = total_steps - active_drag.applied_steps;
                        if step_delta != 0 {
                            let digit_index = active_drag.digit_index;
                            let any_change = apply_drag_step_delta(
                                self.value,
                                digits,
                                digit_index,
                                step_delta,
                                clamped_max,
                            );
                            active_drag.applied_steps = total_steps;
                            drag_focus_digit = Some(digit_index);
                            state.has_saved_selection = true;
                            request_editor_focus = true;
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                            if any_change {
                                changed = true;
                                action = Some(DigitwiseNumberEditorAction::DragAdjustPlace);
                            }
                        }
                    }
                }

                drag_state = Some(active_drag);
            } else {
                drag_focus_digit = Some(active_drag.digit_index);
                drag_state = None;
            }
        }

        let focused_digit = rendered_digits
            .first()
            .and(ui.memory(|memory| memory.has_focus(editor_id)).then_some(
                if state.has_saved_selection {
                    state.selected_digit
                } else {
                    first_significant_digit_index(&digit_chars)
                },
            ))
            .or(drag_focus_digit)
            .or(request_editor_focus.then_some(state.selected_digit));

        if let Some(focused_digit) = focused_digit {
            state.selected_digit = focused_digit;
            state.has_focus = true;

            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    editor_id,
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
                state.has_saved_selection = true;
                action = Some(DigitwiseNumberEditorAction::MoveLeft);
            } else if right_presses > 0 && focused_digit + 1 < digits {
                state.selected_digit = focused_digit + 1;
                state.has_saved_selection = true;
                action = Some(DigitwiseNumberEditorAction::MoveRight);
            } else if up_presses > 0 {
                let mut any_change = false;
                for _ in 0..up_presses {
                    if apply_step_at_digit(self.value, digits, focused_digit, 1, clamped_max) {
                        any_change = true;
                    }
                }
                if any_change {
                    changed = true;
                    state.has_saved_selection = true;
                    action = Some(DigitwiseNumberEditorAction::IncrementPlace);
                }
            } else if down_presses > 0 {
                let mut any_change = false;
                for _ in 0..down_presses {
                    if apply_step_at_digit(self.value, digits, focused_digit, -1, clamped_max) {
                        any_change = true;
                    }
                }
                if any_change {
                    changed = true;
                    state.has_saved_selection = true;
                    action = Some(DigitwiseNumberEditorAction::DecrementPlace);
                }
            } else if let Some(input) = typed_digit_input(ui) {
                let current_digit =
                    digit_chars[focused_digit].to_digit(10).expect("digit char") as u8;

                if let Some(new_digit) = input {
                    let next_digit = (focused_digit + 1).min(digits - 1);
                    if new_digit == current_digit {
                        if next_digit != focused_digit {
                            state.selected_digit = next_digit;
                            state.has_saved_selection = true;
                            action = Some(DigitwiseNumberEditorAction::MoveRight);
                        }
                    } else if apply_replace_digit(
                        self.value,
                        digits,
                        focused_digit,
                        new_digit,
                        clamped_max,
                    ) {
                        changed = true;
                        action = Some(DigitwiseNumberEditorAction::ReplaceDigit);
                        state.selected_digit = next_digit;
                        state.has_saved_selection = true;
                    }
                }
            }
        } else {
            state.has_focus = false;
        }

        if request_editor_focus {
            ui.memory_mut(|memory| memory.request_focus(editor_id));
            state.has_focus = true;
        }

        store_state(ui.ctx(), self.id_source, state);
        store_drag_state(ui.ctx(), drag_state_id, drag_state);

        DigitwiseNumberEditorOutput {
            response,
            changed,
            selected_digit: state.selected_digit,
            action,
        }
    }
}

pub fn focused_widget_is_digitwise_editor(ctx: &egui::Context) -> bool {
    let Some(focused_id) = ctx.memory(|memory| memory.focused()) else {
        return false;
    };
    ctx.data(|data| {
        data.get_temp::<Vec<egui::Id>>(egui::Id::new(DIGITWISE_EDITOR_IDS_DATA_KEY))
            .is_some_and(|ids| ids.contains(&focused_id))
    })
}

fn load_state(ctx: &egui::Context, id: egui::Id) -> EditorState {
    ctx.data_mut(|data| data.get_temp(id)).unwrap_or_default()
}

fn register_digit_ids(ctx: &egui::Context, ids: impl IntoIterator<Item = egui::Id>) {
    let data_id = egui::Id::new(DIGITWISE_EDITOR_IDS_DATA_KEY);
    ctx.data_mut(|data| {
        let mut known_ids = data.get_temp::<Vec<egui::Id>>(data_id).unwrap_or_default();
        for id in ids {
            if !known_ids.contains(&id) {
                known_ids.push(id);
            }
        }
        data.insert_temp(data_id, known_ids);
    });
}

fn store_state(ctx: &egui::Context, id: egui::Id, state: EditorState) {
    ctx.data_mut(|data| data.insert_temp(id, state));
}

fn load_drag_state(ctx: &egui::Context, id: egui::Id) -> Option<DragState> {
    ctx.data_mut(|data| data.get_temp(id))
}

fn store_drag_state(ctx: &egui::Context, id: egui::Id, state: Option<DragState>) {
    ctx.data_mut(|data| {
        if let Some(state) = state {
            data.insert_temp(id, state);
        } else {
            data.remove::<DragState>(id);
        }
    });
}

fn digit_size(ui: &Ui, digit_width: Option<f32>) -> egui::Vec2 {
    let glyph_size = glyph_size(ui);
    egui::vec2(
        digit_width.unwrap_or(glyph_size.x + 4.0),
        glyph_size.y + 4.0,
    )
}

fn digit_interaction_sense() -> Sense {
    Sense {
        click: true,
        drag: true,
        focusable: false,
    }
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

fn paint_digit(
    ui: &Ui,
    rect: egui::Rect,
    digit_char: char,
    font_id: &FontId,
    has_focus: bool,
    is_leading_zero: bool,
) {
    let base_bg = ui.visuals().extreme_bg_color;
    let bg_fill = if has_focus {
        base_bg.linear_multiply(5.0)
    } else {
        base_bg.linear_multiply(0.9)
    };
    let text_color = if is_leading_zero && !has_focus {
        ui.visuals().weak_text_color()
    } else {
        ui.visuals().text_color()
    };

    ui.painter()
        .rect(rect, 1.5, bg_fill, Stroke::new(0.0, Color32::TRANSPARENT));
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        digit_char,
        font_id.clone(),
        text_color,
    );
}

fn paint_separator(ui: &mut Ui, font_id: &FontId) {
    let glyph_size = glyph_size(ui);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(
            glyph_size.x * GROUP_SEPARATOR_WIDTH_FACTOR,
            glyph_size.y + 4.0,
        ),
        Sense::hover(),
    );
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

fn is_leading_zero_digit(digit_chars: &[char], digit_index: usize) -> bool {
    digit_chars.get(digit_index) == Some(&'0')
        && digit_chars
            .iter()
            .take(digit_index + 1)
            .all(|digit_char| *digit_char == '0')
}

fn first_significant_digit_index(digit_chars: &[char]) -> usize {
    digit_chars
        .iter()
        .position(|digit_char| *digit_char != '0')
        .unwrap_or(digit_chars.len().saturating_sub(1))
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

fn apply_drag_step_delta(
    value: &mut u64,
    digits: usize,
    digit_index: usize,
    step_delta: i32,
    max: u64,
) -> bool {
    let delta_sign = step_delta.signum() as i8;
    let nr_steps = step_delta.unsigned_abs();
    let mut any_change = false;
    for _ in 0..nr_steps {
        if apply_step_at_digit(value, digits, digit_index, delta_sign, max) {
            any_change = true;
        }
    }
    any_change
}

fn drag_steps_from_pointer(press_y: f32, current_y: f32) -> i32 {
    let delta = press_y - current_y;
    if delta.abs() < DRAG_START_THRESHOLD_PX {
        return 0;
    }
    (delta / DRAG_STEP_PX).trunc() as i32
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
    fn drag_steps_require_threshold_and_follow_vertical_direction() {
        assert_eq!(drag_steps_from_pointer(100.0, 97.0), 0);
        assert_eq!(drag_steps_from_pointer(100.0, 88.0), 1);
        assert_eq!(drag_steps_from_pointer(100.0, 76.0), 2);
        assert_eq!(drag_steps_from_pointer(100.0, 112.0), -1);
        assert_eq!(drag_steps_from_pointer(100.0, 124.0), -2);
    }

    #[test]
    fn drag_steps_are_stable_within_same_step_band() {
        assert_eq!(drag_steps_from_pointer(100.0, 89.0), 0);
        assert_eq!(drag_steps_from_pointer(100.0, 88.1), 0);
        assert_eq!(drag_steps_from_pointer(100.0, 88.0), 1);
        assert_eq!(drag_steps_from_pointer(100.0, 79.0), 1);
    }

    #[test]
    fn format_value_keeps_leading_zeroes() {
        assert_eq!(format_value(42, 6), "000042");
    }

    #[test]
    fn leading_zero_detection_marks_prefix_zeroes_only() {
        let digits: Vec<char> = "000042".chars().collect();
        assert!(is_leading_zero_digit(&digits, 0));
        assert!(is_leading_zero_digit(&digits, 3));
        assert!(!is_leading_zero_digit(&digits, 4));
        assert!(!is_leading_zero_digit(&digits, 5));
    }

    #[test]
    fn leading_zero_detection_marks_all_zero_value_digits() {
        let digits: Vec<char> = "000000".chars().collect();
        for index in 0..digits.len() {
            assert!(is_leading_zero_digit(&digits, index));
        }
    }

    #[test]
    fn leading_zero_detection_ignores_non_zero_prefixes() {
        let digits: Vec<char> = "120000".chars().collect();
        for index in 0..digits.len() {
            assert!(!is_leading_zero_digit(&digits, index));
        }
    }

    #[test]
    fn first_significant_digit_skips_leading_zeroes() {
        let digits: Vec<char> = "000042".chars().collect();
        assert_eq!(first_significant_digit_index(&digits), 4);
    }

    #[test]
    fn first_significant_digit_uses_last_digit_for_zero_value() {
        let digits: Vec<char> = "000000".chars().collect();
        assert_eq!(first_significant_digit_index(&digits), 5);
    }

    #[test]
    fn first_significant_digit_uses_leftmost_non_zero_when_present() {
        let digits: Vec<char> = "120000".chars().collect();
        assert_eq!(first_significant_digit_index(&digits), 0);
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
