//! Keybindings configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Modifier keys
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// Control key
    Ctrl,
    /// Alt/Option key
    Alt,
    /// Shift key
    Shift,
    /// Super/Windows/Command key
    Super,
}

/// A key combination (key + optional modifiers)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    /// The key (e.g., "c", "Enter", "Tab", "F1")
    pub key: String,
    /// Modifier keys
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
}

impl KeyCombo {
    /// Create a new key combination
    pub fn new(key: impl Into<String>, modifiers: Vec<Modifier>) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    /// Create a key combination with Ctrl modifier
    pub fn ctrl(key: impl Into<String>) -> Self {
        Self::new(key, vec![Modifier::Ctrl])
    }

    /// Create a key combination with Alt modifier
    pub fn alt(key: impl Into<String>) -> Self {
        Self::new(key, vec![Modifier::Alt])
    }

    /// Create a key combination with Shift modifier
    pub fn shift(key: impl Into<String>) -> Self {
        Self::new(key, vec![Modifier::Shift])
    }

    /// Create a key combination with Ctrl+Shift modifiers
    pub fn ctrl_shift(key: impl Into<String>) -> Self {
        Self::new(key, vec![Modifier::Ctrl, Modifier::Shift])
    }

    /// Create a key combination with Alt+Shift modifiers
    pub fn alt_shift(key: impl Into<String>) -> Self {
        Self::new(key, vec![Modifier::Alt, Modifier::Shift])
    }
}

/// Terminal actions that can be bound to keys
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Copy selected text to clipboard
    Copy,
    /// Paste from clipboard
    Paste,
    /// Open a new tab
    NewTab,
    /// Close current tab
    CloseTab,
    /// Split pane horizontally
    SplitHorizontal,
    /// Split pane vertically
    SplitVertical,
    /// Focus next pane/tab
    FocusNext,
    /// Focus previous pane/tab
    FocusPrev,
    /// Open search
    Search,
    /// Increase font size
    IncreaseFontSize,
    /// Decrease font size
    DecreaseFontSize,
    /// Reset font size to default
    ResetFontSize,
    /// Scroll up one line
    ScrollUp,
    /// Scroll down one line
    ScrollDown,
    /// Scroll up one page
    ScrollPageUp,
    /// Scroll down one page
    ScrollPageDown,
    /// Scroll to top of scrollback
    ScrollToTop,
    /// Scroll to bottom of scrollback
    ScrollToBottom,
    /// Toggle fullscreen
    ToggleFullscreen,
    /// Quit the application
    Quit,
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    /// Map of key combinations to actions
    #[serde(default = "default_keybindings")]
    pub bindings: HashMap<KeyCombo, Action>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            bindings: default_keybindings(),
        }
    }
}

fn default_keybindings() -> HashMap<KeyCombo, Action> {
    let mut bindings = HashMap::new();

    // Clipboard
    bindings.insert(KeyCombo::ctrl_shift("c"), Action::Copy);
    bindings.insert(KeyCombo::ctrl_shift("v"), Action::Paste);

    // Tab management
    bindings.insert(KeyCombo::ctrl_shift("t"), Action::NewTab);
    bindings.insert(KeyCombo::ctrl_shift("w"), Action::CloseTab);

    // Pane splitting
    bindings.insert(KeyCombo::ctrl_shift("d"), Action::SplitHorizontal);
    bindings.insert(KeyCombo::ctrl_shift("e"), Action::SplitVertical);

    // Focus navigation
    bindings.insert(KeyCombo::ctrl_shift("right"), Action::FocusNext);
    bindings.insert(KeyCombo::ctrl_shift("Tab"), Action::FocusPrev);
    bindings.insert(KeyCombo::alt("right"), Action::FocusNext);
    bindings.insert(KeyCombo::alt("left"), Action::FocusPrev);

    // Search
    bindings.insert(KeyCombo::ctrl_shift("f"), Action::Search);

    // Font size
    bindings.insert(KeyCombo::ctrl("plus"), Action::IncreaseFontSize);
    bindings.insert(KeyCombo::ctrl("minus"), Action::DecreaseFontSize);
    bindings.insert(KeyCombo::ctrl("0"), Action::ResetFontSize);

    // Scrolling
    bindings.insert(KeyCombo::shift("PageUp"), Action::ScrollPageUp);
    bindings.insert(KeyCombo::shift("PageDown"), Action::ScrollPageDown);
    bindings.insert(KeyCombo::shift("Home"), Action::ScrollToTop);
    bindings.insert(KeyCombo::shift("End"), Action::ScrollToBottom);

    // Window
    bindings.insert(KeyCombo::alt("F11"), Action::ToggleFullscreen);
    bindings.insert(KeyCombo::ctrl_shift("q"), Action::Quit);

    bindings
}

impl Keybindings {
    /// Create a new keybindings configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the action for a key combination
    pub fn get_action(&self, combo: &KeyCombo) -> Option<&Action> {
        self.bindings.get(combo)
    }

    /// Set a keybinding (overwrites existing)
    pub fn set(&mut self, combo: KeyCombo, action: Action) {
        self.bindings.insert(combo, action);
    }

    /// Remove a keybinding
    pub fn remove(&mut self, combo: &KeyCombo) -> Option<Action> {
        self.bindings.remove(combo)
    }

    /// Get all keybindings for an action
    pub fn get_keys_for_action(&self, action: &Action) -> Vec<&KeyCombo> {
        self.bindings
            .iter()
            .filter(|(_, a)| *a == action)
            .map(|(k, _)| k)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_combo_creation() {
        let combo = KeyCombo::ctrl("c");
        assert_eq!(combo.key, "c");
        assert_eq!(combo.modifiers, vec![Modifier::Ctrl]);
    }

    #[test]
    fn test_default_keybindings() {
        let kb = Keybindings::default();

        // Check some default bindings exist
        assert_eq!(
            kb.get_action(&KeyCombo::ctrl_shift("c")),
            Some(&Action::Copy)
        );
        assert_eq!(
            kb.get_action(&KeyCombo::ctrl_shift("v")),
            Some(&Action::Paste)
        );
        assert_eq!(
            kb.get_action(&KeyCombo::ctrl_shift("t")),
            Some(&Action::NewTab)
        );
    }

    #[test]
    fn test_set_and_remove_binding() {
        let mut kb = Keybindings::default();

        // Set a new binding
        kb.set(KeyCombo::alt("x"), Action::Quit);
        assert_eq!(kb.get_action(&KeyCombo::alt("x")), Some(&Action::Quit));

        // Remove the binding
        let removed = kb.remove(&KeyCombo::alt("x"));
        assert_eq!(removed, Some(Action::Quit));
        assert_eq!(kb.get_action(&KeyCombo::alt("x")), None);
    }

    #[test]
    fn test_get_keys_for_action() {
        let kb = Keybindings::default();
        let keys = kb.get_keys_for_action(&Action::FocusNext);
        assert!(!keys.is_empty());
    }

    #[test]
    fn test_key_combo_serialization() {
        let combo = KeyCombo::ctrl_shift("c");
        let toml_str = toml::to_string(&combo).unwrap();

        let parsed: KeyCombo = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed, combo);
    }

    #[test]
    fn test_action_serialization() {
        let action = Action::Copy;
        let toml_str = toml::to_string(&action).unwrap();
        assert_eq!(toml_str.trim(), "\"copy\"");

        let parsed: Action = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed, action);
    }

    #[test]
    fn test_keybindings_serialization() {
        let kb = Keybindings::default();
        let toml_str = toml::to_string_pretty(&kb).unwrap();

        // Should contain some bindings
        assert!(toml_str.contains("[bindings]"));

        // Parse back
        let parsed: Keybindings = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            parsed.get_action(&KeyCombo::ctrl_shift("c")),
            Some(&Action::Copy)
        );
    }
}
