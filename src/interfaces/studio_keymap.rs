//! Keyboard shortcuts: the actions Studio offers, their default keys and the person's changes.
//!
//! A binding is text such as `mod+shift+f` (`mod` is Ctrl, or Cmd on macOS). The effective
//! bindings are global because the key listener is a plain function.
use iced::keyboard::{Key, Modifiers, key::Named};
use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    ToggleSidebar,
    ToggleDock,
    SearchProject,
    Save,
    CloseTab,
    OpenSettings,
    NewChat,
    CopyLastAnswer,
}

impl Action {
    pub const ALL: [Action; 8] = [
        Action::ToggleSidebar,
        Action::ToggleDock,
        Action::SearchProject,
        Action::Save,
        Action::CloseTab,
        Action::OpenSettings,
        Action::NewChat,
        Action::CopyLastAnswer,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Action::ToggleSidebar => "toggle-sidebar",
            Action::ToggleDock => "toggle-dock",
            Action::SearchProject => "search-project",
            Action::Save => "save",
            Action::CloseTab => "close-tab",
            Action::OpenSettings => "settings",
            Action::NewChat => "new-chat",
            Action::CopyLastAnswer => "copy-last-answer",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::ToggleSidebar => "Mostrar u ocultar la barra lateral",
            Action::ToggleDock => "Mostrar u ocultar el panel inferior",
            Action::SearchProject => "Buscar en el proyecto",
            Action::Save => "Guardar el archivo",
            Action::CloseTab => "Cerrar la pestaña",
            Action::OpenSettings => "Abrir los ajustes",
            Action::NewChat => "Nuevo chat (en el chat activo)",
            Action::CopyLastAnswer => "Copiar la última respuesta del agente",
        }
    }

    pub fn default_binding(self) -> &'static str {
        match self {
            Action::ToggleSidebar => "mod+b",
            Action::ToggleDock => "mod+j",
            Action::SearchProject => "mod+shift+f",
            Action::Save => "mod+s",
            Action::CloseTab => "mod+w",
            Action::OpenSettings => "mod+,",
            Action::NewChat => "mod+shift+n",
            Action::CopyLastAnswer => "mod+shift+y",
        }
    }

    /// Saving and closing leave the key to a focused editor that already used it.
    pub fn needs_free_key(self) -> bool {
        matches!(self, Action::Save | Action::CloseTab)
    }
}

static BINDINGS: RwLock<Vec<(Action, String)>> = RwLock::new(Vec::new());
static CAPTURING: AtomicBool = AtomicBool::new(false);

/// Applies the person's changes over the defaults (an empty binding removes the shortcut).
pub(super) fn apply(overrides: &BTreeMap<String, String>) {
    let table = Action::ALL
        .into_iter()
        .map(|action| {
            let binding = overrides
                .get(action.id())
                .cloned()
                .unwrap_or_else(|| action.default_binding().to_owned());
            (action, binding)
        })
        .collect();
    if let Ok(mut slot) = BINDINGS.write() {
        *slot = table;
    }
}

pub(super) fn binding(action: Action) -> String {
    BINDINGS
        .read()
        .ok()
        .and_then(|table| {
            table
                .iter()
                .find(|(candidate, _)| *candidate == action)
                .map(|(_, binding)| binding.clone())
        })
        .unwrap_or_else(|| action.default_binding().to_owned())
}

pub(super) fn lookup(chord: &str) -> Option<Action> {
    let table = BINDINGS.read().ok()?;
    table
        .iter()
        .find(|(_, binding)| !binding.is_empty() && binding == chord)
        .map(|(action, _)| *action)
}

/// While a shortcut is being recorded the keys belong to the recorder, not to the actions.
pub(super) fn set_capturing(on: bool) {
    CAPTURING.store(on, Ordering::Relaxed);
}

pub(super) fn capturing() -> bool {
    CAPTURING.load(Ordering::Relaxed)
}

/// The binding a key press makes, or `None` for a lone modifier or an unsupported key.
pub(super) fn chord(key: &Key, modifiers: Modifiers) -> Option<String> {
    let name = match key.as_ref() {
        Key::Character(character) => character.to_lowercase(),
        Key::Named(named) => named_key(named)?.to_owned(),
        _ => return None,
    };
    let mut parts = Vec::new();
    if modifiers.command() {
        parts.push("mod");
    }
    if modifiers.alt() {
        parts.push("alt");
    }
    if modifiers.shift() {
        parts.push("shift");
    }
    parts.push(&name);
    Some(parts.join("+"))
}

fn named_key(named: Named) -> Option<&'static str> {
    Some(match named {
        Named::F1 => "f1",
        Named::F2 => "f2",
        Named::F3 => "f3",
        Named::F4 => "f4",
        Named::F5 => "f5",
        Named::F6 => "f6",
        Named::F7 => "f7",
        Named::F8 => "f8",
        Named::F9 => "f9",
        Named::F10 => "f10",
        Named::F11 => "f11",
        Named::F12 => "f12",
        Named::Enter => "enter",
        Named::Tab => "tab",
        Named::ArrowUp => "up",
        Named::ArrowDown => "down",
        Named::ArrowLeft => "left",
        Named::ArrowRight => "right",
        Named::PageUp => "pageup",
        Named::PageDown => "pagedown",
        Named::Home => "home",
        Named::End => "end",
        _ => return None,
    })
}

/// A shortcut needs Ctrl/Cmd or Alt (or to be a function key), so typing never triggers it.
pub(super) fn is_bindable(chord: &str) -> bool {
    chord.starts_with("mod+")
        || chord.starts_with("alt+")
        || chord
            .strip_prefix('f')
            .is_some_and(|n| n.parse::<u8>().is_ok())
}

/// "mod+shift+f" as people read it: "Ctrl+Shift+F" (or "⌘⇧F" on macOS).
pub(super) fn display(binding: &str) -> String {
    if binding.is_empty() {
        return "Sin atajo".to_owned();
    }
    binding
        .split('+')
        .map(|part| match part {
            "mod" if cfg!(target_os = "macos") => "⌘".to_owned(),
            "mod" => "Ctrl".to_owned(),
            "alt" => "Alt".to_owned(),
            "shift" => "Mayús".to_owned(),
            "up" => "↑".to_owned(),
            "down" => "↓".to_owned(),
            "left" => "←".to_owned(),
            "right" => "→".to_owned(),
            other => {
                let mut chars = other.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            }
        })
        .collect::<Vec<_>>()
        .join(if cfg!(target_os = "macos") { "" } else { "+" })
}

/// The action that already uses `chord`, other than `except`.
pub(super) fn owner(chord: &str, except: Action) -> Option<Action> {
    lookup(chord).filter(|action| *action != except)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords_are_read_back_and_changes_replace_the_defaults() {
        apply(&BTreeMap::new());
        assert_eq!(lookup("mod+b"), Some(Action::ToggleSidebar));
        let mut changes = BTreeMap::new();
        changes.insert("toggle-sidebar".to_owned(), "alt+b".to_owned());
        changes.insert("save".to_owned(), String::new());
        apply(&changes);
        assert_eq!(lookup("alt+b"), Some(Action::ToggleSidebar));
        assert_eq!(lookup("mod+b"), None);
        assert_eq!(lookup("mod+s"), None);
        assert_eq!(owner("alt+b", Action::Save), Some(Action::ToggleSidebar));
        apply(&BTreeMap::new());
    }

    #[test]
    fn only_chords_with_a_modifier_or_a_function_key_can_be_bound() {
        assert!(is_bindable("mod+shift+n") && is_bindable("alt+x") && is_bindable("f5"));
        assert!(!is_bindable("a") && !is_bindable("shift+a") && !is_bindable("enter"));
        assert_eq!(display("mod+shift+f"), "Ctrl+Mayús+F");
    }
}
