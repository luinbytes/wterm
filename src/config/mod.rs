//! Configuration management

pub mod keybindings;
pub mod settings;
pub mod theme;

pub use keybindings::{Action, KeyCombo, Keybindings, Modifier};
pub use settings::{Config, FontConfig, TerminalConfig, WindowConfig};
pub use theme::{AnsiColors, Color, Theme, ThemeConfig};
