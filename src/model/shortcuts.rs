use crate::model::{Action, tracks2::Tracks};
use egui::{Key, KeyboardShortcut, Modifiers};
use egui_custom_widgets::focused_widget_is_digitwise_editor;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    ZoomToSelection,
    ZoomToFull,
    FillScreenHeight,
    RecenterYAll,
}

impl ShortcutAction {
    pub const ALL: [Self; 4] = [
        Self::ZoomToSelection,
        Self::ZoomToFull,
        Self::FillScreenHeight,
        Self::RecenterYAll,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ZoomToSelection => "Zoom To Selection",
            Self::ZoomToFull => "Reset X Zoom",
            Self::FillScreenHeight => "Fill Screen Height",
            Self::RecenterYAll => "Recenter Y",
        }
    }

    pub fn to_model_action(self) -> Action {
        match self {
            Self::ZoomToSelection => Action::ZoomToSelection,
            Self::ZoomToFull => Action::ZoomToFull,
            Self::FillScreenHeight => Action::FillScreenHeight,
            Self::RecenterYAll => Action::RecenterYAll,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutScope {
    #[default]
    Global,
    OneHand,
}

impl ShortcutScope {
    pub const ALL: [Self; 2] = [Self::Global, Self::OneHand];

    pub fn label(self) -> &'static str {
        match self {
            Self::Global => "Modifier shortcuts",
            Self::OneHand => "One-hand shortcuts",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ShortcutBinding {
    pub action: ShortcutAction,
    pub scope: ShortcutScope,
    pub key: String,
    pub command: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl ShortcutBinding {
    pub fn new(action: ShortcutAction, key: impl Into<String>) -> Self {
        Self {
            action,
            scope: ShortcutScope::Global,
            key: key.into(),
            command: false,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn with_command(mut self) -> Self {
        self.command = true;
        self
    }

    pub fn with_scope(mut self, scope: ShortcutScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn to_keyboard_shortcut(&self) -> Option<KeyboardShortcut> {
        let key = Key::from_name(self.key.trim())?;
        Some(KeyboardShortcut::new(
            Modifiers {
                alt: self.alt,
                ctrl: self.ctrl,
                shift: self.shift,
                mac_cmd: false,
                command: self.command,
            },
            key,
        ))
    }

    pub fn specificity(&self) -> usize {
        usize::from(self.command)
            + usize::from(self.ctrl)
            + usize::from(self.shift)
            + usize::from(self.alt)
    }

    pub fn formatted(&self, ctx: &egui::Context) -> String {
        if let Some(shortcut) = self.to_keyboard_shortcut() {
            ctx.format_shortcut(&shortcut)
        } else {
            let mut parts = Vec::new();
            if self.command {
                parts.push("Cmd/Ctrl".to_string());
            }
            if self.ctrl {
                parts.push("Ctrl".to_string());
            }
            if self.alt {
                parts.push("Alt".to_string());
            }
            if self.shift {
                parts.push("Shift".to_string());
            }
            parts.push(self.key.clone());
            parts.join("+")
        }
    }
}

impl Default for ShortcutBinding {
    fn default() -> Self {
        ShortcutConfig::default_binding_for(ShortcutAction::ZoomToSelection, ShortcutScope::Global)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ShortcutConfig {
    pub bindings: Vec<ShortcutBinding>,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            bindings: ShortcutAction::ALL
                .into_iter()
                .flat_map(|action| {
                    ShortcutScope::ALL
                        .into_iter()
                        .map(move |scope| Self::default_binding_for(action, scope))
                })
                .collect(),
        }
    }
}

impl ShortcutConfig {
    pub fn default_binding_for(action: ShortcutAction, scope: ShortcutScope) -> ShortcutBinding {
        match (action, scope) {
            (ShortcutAction::ZoomToSelection, ShortcutScope::Global) => {
                ShortcutBinding::new(action, "L")
                    .with_scope(scope)
                    .with_command()
                    .with_shift()
            }
            (ShortcutAction::ZoomToSelection, ShortcutScope::OneHand) => {
                ShortcutBinding::new(action, "S").with_scope(scope)
            }
            (ShortcutAction::ZoomToFull, ShortcutScope::Global) => {
                ShortcutBinding::new(action, "0")
                    .with_scope(scope)
                    .with_command()
            }
            (ShortcutAction::ZoomToFull, ShortcutScope::OneHand) => {
                ShortcutBinding::new(action, "R").with_scope(scope)
            }
            (ShortcutAction::FillScreenHeight, ShortcutScope::Global) => {
                ShortcutBinding::new(action, "H")
                    .with_scope(scope)
                    .with_command()
                    .with_shift()
            }
            (ShortcutAction::FillScreenHeight, ShortcutScope::OneHand) => {
                ShortcutBinding::new(action, "H").with_scope(scope)
            }
            (ShortcutAction::RecenterYAll, ShortcutScope::Global) => {
                ShortcutBinding::new(action, "Y")
                    .with_scope(scope)
                    .with_command()
                    .with_shift()
            }
            (ShortcutAction::RecenterYAll, ShortcutScope::OneHand) => {
                ShortcutBinding::new(action, "Y").with_scope(scope)
            }
        }
    }

    pub fn normalize(&mut self) {
        let mut normalized =
            Vec::with_capacity(ShortcutAction::ALL.len() * ShortcutScope::ALL.len());
        for action in ShortcutAction::ALL {
            for scope in ShortcutScope::ALL {
                let mut bindings = self
                    .bindings
                    .iter()
                    .filter(|binding| binding.action == action && binding.scope == scope)
                    .map(|binding| Self::migrate_legacy_default(binding.clone()));
                let mut chosen = None;
                let mut invalid_count = 0usize;
                while let Some(binding) = bindings.next() {
                    if binding.to_keyboard_shortcut().is_some() {
                        chosen = Some(binding.clone());
                        if bindings.next().is_some() {
                            warn!(
                                ?action,
                                ?scope,
                                "Duplicate shortcut bindings found; keeping the first valid one"
                            );
                        }
                        break;
                    }
                    invalid_count += 1;
                }
                if invalid_count > 0 {
                    warn!(
                        ?action,
                        ?scope,
                        invalid_count,
                        "Invalid shortcut bindings found; falling back to defaults"
                    );
                }
                normalized.push(chosen.unwrap_or_else(|| Self::default_binding_for(action, scope)));
            }
        }
        self.bindings = normalized;
    }

    fn migrate_legacy_default(binding: ShortcutBinding) -> ShortcutBinding {
        if binding == legacy_zoom_to_selection_binding("Z")
            || binding == legacy_zoom_to_selection_binding("X")
        {
            return Self::default_binding_for(
                ShortcutAction::ZoomToSelection,
                ShortcutScope::Global,
            );
        }
        if binding == legacy_fill_screen_height_binding("F") {
            return Self::default_binding_for(
                ShortcutAction::FillScreenHeight,
                ShortcutScope::Global,
            );
        }
        binding
    }
}

fn legacy_zoom_to_selection_binding(key: &str) -> ShortcutBinding {
    ShortcutBinding::new(ShortcutAction::ZoomToSelection, key)
        .with_scope(ShortcutScope::Global)
        .with_command()
        .with_shift()
}

fn legacy_fill_screen_height_binding(key: &str) -> ShortcutBinding {
    ShortcutBinding::new(ShortcutAction::FillScreenHeight, key)
        .with_scope(ShortcutScope::Global)
        .with_command()
        .with_shift()
}

pub fn dispatch_shortcuts(
    ctx: &egui::Context,
    tracks: &Tracks,
    shortcut_config: &ShortcutConfig,
    actions: &mut Vec<Action>,
) {
    dispatch_global_shortcuts(ctx, shortcut_config, actions);

    if navigation_surface_hovered(ctx, tracks) {
        dispatch_scoped_shortcuts(shortcut_config, ShortcutScope::OneHand, ctx, actions);
    }
}

fn dispatch_global_shortcuts(
    ctx: &egui::Context,
    shortcut_config: &ShortcutConfig,
    actions: &mut Vec<Action>,
) {
    if ctx.wants_keyboard_input() && !focused_widget_is_digitwise_editor(ctx) {
        return;
    }
    dispatch_scoped_shortcuts(shortcut_config, ShortcutScope::Global, ctx, actions);
}

fn dispatch_scoped_shortcuts(
    shortcut_config: &ShortcutConfig,
    scope: ShortcutScope,
    ctx: &egui::Context,
    actions: &mut Vec<Action>,
) {
    let mut bindings: Vec<_> = shortcut_config
        .bindings
        .iter()
        .filter(|binding| binding.scope == scope)
        .filter_map(|binding| {
            binding
                .to_keyboard_shortcut()
                .map(|shortcut| (binding, shortcut))
        })
        .collect();
    bindings.sort_by_key(|(binding, _)| std::cmp::Reverse(binding.specificity()));

    ctx.input_mut(|input| {
        for (binding, shortcut) in &bindings {
            if input.consume_shortcut(shortcut) {
                actions.push(binding.action.to_model_action());
            }
        }
    });
}

fn navigation_surface_hovered(ctx: &egui::Context, tracks: &Tracks) -> bool {
    let Some(pointer_pos) = ctx.pointer_hover_pos() else {
        return false;
    };
    if tracks.ruler.screen_rect().contains(pointer_pos.into()) {
        return true;
    }
    tracks.tracks.values().any(|track| {
        track
            .screen_rect
            .is_some_and(|rect| rect.contains(pointer_pos.into()))
    })
}

#[cfg(test)]
mod tests {
    use super::{ShortcutAction, ShortcutBinding, ShortcutConfig, ShortcutScope};
    use crate::model::{Action, tracks2::Tracks};
    use egui::{Context, Event, Key, Modifiers, RawInput};

    #[test]
    fn default_config_has_global_and_one_hand_bindings() {
        let config = ShortcutConfig::default();

        assert_eq!(
            config.bindings.len(),
            ShortcutAction::ALL.len() * ShortcutScope::ALL.len()
        );
    }

    #[test]
    fn normalize_falls_back_to_default_for_invalid_key() {
        let mut config = ShortcutConfig {
            bindings: vec![ShortcutBinding {
                action: ShortcutAction::ZoomToSelection,
                scope: ShortcutScope::Global,
                key: "NotAKey".into(),
                command: true,
                ctrl: false,
                shift: false,
                alt: false,
            }],
        };

        config.normalize();

        assert_eq!(config.bindings.len(), 8);
        assert_eq!(
            config.bindings[0],
            ShortcutConfig::default_binding_for(
                ShortcutAction::ZoomToSelection,
                ShortcutScope::Global,
            )
        );
    }

    #[test]
    fn normalize_keeps_first_valid_binding_per_action_and_scope() {
        let mut config = ShortcutConfig {
            bindings: vec![
                ShortcutBinding::new(ShortcutAction::ZoomToFull, "1")
                    .with_scope(ShortcutScope::Global)
                    .with_command(),
                ShortcutBinding::new(ShortcutAction::ZoomToFull, "2")
                    .with_scope(ShortcutScope::Global)
                    .with_command(),
            ],
        };

        config.normalize();

        assert_eq!(
            config.bindings[2],
            ShortcutBinding::new(ShortcutAction::ZoomToFull, "1")
                .with_scope(ShortcutScope::Global)
                .with_command()
        );
    }

    #[test]
    fn default_global_bindings_match_expected() {
        assert_eq!(
            ShortcutConfig::default_binding_for(
                ShortcutAction::ZoomToSelection,
                ShortcutScope::Global,
            ),
            ShortcutBinding::new(ShortcutAction::ZoomToSelection, "L")
                .with_scope(ShortcutScope::Global)
                .with_command()
                .with_shift()
        );
        assert_eq!(
            ShortcutConfig::default_binding_for(
                ShortcutAction::FillScreenHeight,
                ShortcutScope::Global,
            ),
            ShortcutBinding::new(ShortcutAction::FillScreenHeight, "H")
                .with_scope(ShortcutScope::Global)
                .with_command()
                .with_shift()
        );
    }

    #[test]
    fn default_one_hand_bindings_match_expected() {
        assert_eq!(
            ShortcutConfig::default_binding_for(
                ShortcutAction::ZoomToSelection,
                ShortcutScope::OneHand,
            ),
            ShortcutBinding::new(ShortcutAction::ZoomToSelection, "S")
                .with_scope(ShortcutScope::OneHand)
        );
        assert_eq!(
            ShortcutConfig::default_binding_for(ShortcutAction::ZoomToFull, ShortcutScope::OneHand),
            ShortcutBinding::new(ShortcutAction::ZoomToFull, "R")
                .with_scope(ShortcutScope::OneHand)
        );
    }

    #[test]
    fn normalize_migrates_legacy_zoom_to_selection_bindings() {
        let mut config = ShortcutConfig {
            bindings: vec![
                ShortcutBinding::new(ShortcutAction::ZoomToSelection, "X")
                    .with_scope(ShortcutScope::Global)
                    .with_command()
                    .with_shift(),
            ],
        };

        config.normalize();

        assert_eq!(
            config.bindings[0],
            ShortcutConfig::default_binding_for(
                ShortcutAction::ZoomToSelection,
                ShortcutScope::Global,
            )
        );
    }

    #[test]
    fn normalize_migrates_legacy_fill_screen_height_binding() {
        let mut config = ShortcutConfig {
            bindings: vec![
                ShortcutBinding::new(ShortcutAction::FillScreenHeight, "F")
                    .with_scope(ShortcutScope::Global)
                    .with_command()
                    .with_shift(),
            ],
        };

        config.normalize();

        assert_eq!(
            config.bindings[4],
            ShortcutConfig::default_binding_for(
                ShortcutAction::FillScreenHeight,
                ShortcutScope::Global,
            )
        );
    }

    #[test]
    fn dispatch_allows_global_shortcuts_with_digitwise_editor_focus() {
        let ctx = Context::default();
        let mut actions = Vec::new();
        let shortcut_config = ShortcutConfig::default();
        let focused_id = egui::Id::new("digit_editor_focus");
        let tracks = Tracks::default();

        let _ = ctx.run(
            RawInput {
                events: vec![Event::Key {
                    key: Key::L,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers {
                        command: true,
                        shift: true,
                        ..Modifiers::NONE
                    },
                }],
                ..Default::default()
            },
            |ctx| {
                ctx.memory_mut(|memory| memory.request_focus(focused_id));
                ctx.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new("digitwise_number_editor_ids"),
                        vec![focused_id],
                    );
                });
                super::dispatch_shortcuts(ctx, &tracks, &shortcut_config, &mut actions);
            },
        );

        assert_eq!(actions, vec![Action::ZoomToSelection]);
    }

    #[test]
    fn one_hand_shortcuts_require_navigation_surface_hover() {
        let ctx = Context::default();
        let mut actions = Vec::new();
        let shortcut_config = ShortcutConfig::default();
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(crate::rect::Rect::new(0.0, 0.0, 100.0, 100.0));

        let _ = ctx.run(
            RawInput {
                events: vec![Event::Key {
                    key: Key::S,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ctx| {
                super::dispatch_shortcuts(ctx, &tracks, &shortcut_config, &mut actions);
            },
        );

        assert!(actions.is_empty());
    }

    #[test]
    fn one_hand_shortcuts_fire_when_navigation_surface_is_hovered() {
        let ctx = Context::default();
        let mut actions = Vec::new();
        let shortcut_config = ShortcutConfig::default();
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(crate::rect::Rect::new(0.0, 0.0, 100.0, 100.0));

        let _ = ctx.run(
            RawInput {
                events: vec![
                    Event::PointerMoved(egui::pos2(10.0, 10.0)),
                    Event::Key {
                        key: Key::S,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| {
                super::dispatch_shortcuts(ctx, &tracks, &shortcut_config, &mut actions);
            },
        );

        assert_eq!(actions, vec![Action::ZoomToSelection]);
    }
}
