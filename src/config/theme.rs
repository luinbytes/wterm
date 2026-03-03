//! Color theme configuration
//!
//! This module provides theme types for future config support.
//! Currently unused but kept for integration with the config system.

use serde::{Deserialize, Serialize};

/// RGB color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

#[allow(dead_code)]
impl Color {
    /// Create a new color from RGB values
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create a color from a hex string (e.g., "#1e1e2e")
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#')?;
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Self { r, g, b })
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// ANSI 16-color palette
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct AnsiColors {
    /// Black (color 0)
    pub black: Color,
    /// Red (color 1)
    pub red: Color,
    /// Green (color 2)
    pub green: Color,
    /// Yellow (color 3)
    pub yellow: Color,
    /// Blue (color 4)
    pub blue: Color,
    /// Magenta (color 5)
    pub magenta: Color,
    /// Cyan (color 6)
    pub cyan: Color,
    /// White (color 7)
    pub white: Color,
    /// Bright black (color 8)
    pub bright_black: Color,
    /// Bright red (color 9)
    pub bright_red: Color,
    /// Bright green (color 10)
    pub bright_green: Color,
    /// Bright yellow (color 11)
    pub bright_yellow: Color,
    /// Bright blue (color 12)
    pub bright_blue: Color,
    /// Bright magenta (color 13)
    pub bright_magenta: Color,
    /// Bright cyan (color 14)
    pub bright_cyan: Color,
    /// Bright white (color 15)
    pub bright_white: Color,
}

/// Complete theme definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct Theme {
    /// Theme name
    pub name: String,
    /// Background color
    pub background: Color,
    /// Foreground (text) color
    pub foreground: Color,
    /// Cursor color
    pub cursor: Color,
    /// Selection background color
    #[serde(default = "default_selection_bg")]
    pub selection_background: Color,
    /// Selection foreground color
    #[serde(default = "default_selection_fg")]
    pub selection_foreground: Color,
    /// ANSI 16-color palette
    pub colors: AnsiColors,
}

fn default_selection_bg() -> Color {
    Color::new(0x44, 0x44, 0x44)
}

fn default_selection_fg() -> Color {
    Color::new(0xff, 0xff, 0xff)
}

#[allow(dead_code)]
impl Theme {
    /// Get the default dark theme
    pub fn default_dark() -> Self {
        Self {
            name: "default".to_string(),
            background: Color::new(0x1e, 0x1e, 0x2e),
            foreground: Color::new(0xc6, 0xd0, 0xf5),
            cursor: Color::new(0xf5, 0xe0, 0xdc),
            selection_background: Color::new(0x45, 0x47, 0x5a),
            selection_foreground: Color::new(0xc6, 0xd0, 0xf5),
            colors: AnsiColors {
                black: Color::new(0x18, 0x18, 0x1a),
                red: Color::new(0xf3, 0x8b, 0xa8),
                green: Color::new(0xa6, 0xda, 0x95),
                yellow: Color::new(0xee, 0xd4, 0x9f),
                blue: Color::new(0x8b, 0xa4, 0xbe),
                magenta: Color::new(0xf5, 0xb2, 0xe6),
                cyan: Color::new(0x8b, 0xd5, 0xca),
                white: Color::new(0xb5, 0xbf, 0xeb),
                bright_black: Color::new(0x51, 0x52, 0x5d),
                bright_red: Color::new(0xf3, 0x8b, 0xa8),
                bright_green: Color::new(0xa6, 0xda, 0x95),
                bright_yellow: Color::new(0xee, 0xd4, 0x9f),
                bright_blue: Color::new(0x8b, 0xa4, 0xbe),
                bright_magenta: Color::new(0xf5, 0xb2, 0xe6),
                bright_cyan: Color::new(0x8b, 0xd5, 0xca),
                bright_white: Color::new(0xa5, 0xb0, 0xe8),
            },
        }
    }

    /// Get the Dracula theme
    pub fn dracula() -> Self {
        Self {
            name: "dracula".to_string(),
            background: Color::new(0x28, 0x2a, 0x36),
            foreground: Color::new(0xf8, 0xf8, 0xf2),
            cursor: Color::new(0xbd, 0x93, 0xf9),
            selection_background: Color::new(0x44, 0x47, 0x5a),
            selection_foreground: Color::new(0xf8, 0xf8, 0xf2),
            colors: AnsiColors {
                black: Color::new(0x00, 0x00, 0x00),
                red: Color::new(0xff, 0x55, 0x55),
                green: Color::new(0x50, 0xfa, 0x7b),
                yellow: Color::new(0xf1, 0xfa, 0x8c),
                blue: Color::new(0xbd, 0x93, 0xf9),
                magenta: Color::new(0xff, 0x79, 0xc6),
                cyan: Color::new(0x8b, 0xe9, 0xfd),
                white: Color::new(0xbf, 0xbf, 0xbf),
                bright_black: Color::new(0x4d, 0x4d, 0x4d),
                bright_red: Color::new(0xff, 0x6e, 0x67),
                bright_green: Color::new(0x5a, 0xf7, 0x8e),
                bright_yellow: Color::new(0xf4, 0xf9, 0x9d),
                bright_blue: Color::new(0xca, 0xa9, 0xfa),
                bright_magenta: Color::new(0xff, 0x92, 0xd0),
                bright_cyan: Color::new(0x9a, 0xed, 0xfe),
                bright_white: Color::new(0xe6, 0xe6, 0xe6),
            },
        }
    }

    /// Get the Gruvbox theme
    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox".to_string(),
            background: Color::new(0x28, 0x28, 0x28),
            foreground: Color::new(0xeb, 0xdb, 0xb2),
            cursor: Color::new(0xeb, 0xdb, 0xb2),
            selection_background: Color::new(0x66, 0x5c, 0x54),
            selection_foreground: Color::new(0xeb, 0xdb, 0xb2),
            colors: AnsiColors {
                black: Color::new(0x28, 0x28, 0x28),
                red: Color::new(0xcc, 0x24, 0x1d),
                green: Color::new(0x98, 0x97, 0x1a),
                yellow: Color::new(0xd7, 0x99, 0x21),
                blue: Color::new(0x45, 0x85, 0x88),
                magenta: Color::new(0xb1, 0x62, 0x86),
                cyan: Color::new(0x68, 0x9d, 0x6a),
                white: Color::new(0xa8, 0x99, 0x84),
                bright_black: Color::new(0x92, 0x83, 0x74),
                bright_red: Color::new(0xfb, 0x49, 0x34),
                bright_green: Color::new(0xb8, 0xbb, 0x26),
                bright_yellow: Color::new(0xfa, 0xbd, 0x2f),
                bright_blue: Color::new(0x83, 0xa5, 0x98),
                bright_magenta: Color::new(0xd3, 0x86, 0x9b),
                bright_cyan: Color::new(0x8e, 0xc0, 0x7c),
                bright_white: Color::new(0xeb, 0xdb, 0xb2),
            },
        }
    }

    /// Get a built-in theme by name
    pub fn builtin(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::default_dark()),
            "dracula" => Some(Self::dracula()),
            "gruvbox" => Some(Self::gruvbox()),
            _ => None,
        }
    }

    /// Get all built-in theme names
    pub fn builtin_names() -> &'static [&'static str] {
        &["default", "dracula", "gruvbox"]
    }
}

/// Theme configuration - either a built-in theme name or custom colors
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ThemeConfig {
    /// Name of a built-in theme (default, dracula, gruvbox)
    #[serde(default = "default_theme_name")]
    pub name: String,
    /// Custom theme (overrides built-in if present)
    #[serde(default)]
    pub custom: Option<Theme>,
}

fn default_theme_name() -> String {
    "default".to_string()
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme_name(),
            custom: None,
        }
    }
}

#[allow(dead_code)]
impl ThemeConfig {
    /// Get the effective theme (custom or built-in)
    pub fn resolve(&self) -> Theme {
        if let Some(ref custom) = self.custom {
            custom.clone()
        } else {
            Theme::builtin(&self.name).unwrap_or_else(Theme::default_dark)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#1e1e2e").unwrap();
        assert_eq!(color.r, 0x1e);
        assert_eq!(color.g, 0x1e);
        assert_eq!(color.b, 0x2e);
    }

    #[test]
    fn test_color_to_hex() {
        let color = Color::new(0x1e, 0x1e, 0x2e);
        assert_eq!(color.to_hex(), "#1e1e2e");
    }

    #[test]
    fn test_color_roundtrip() {
        let original = Color::new(0xab, 0xcd, 0xef);
        let hex = original.to_hex();
        let parsed = Color::from_hex(&hex).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_builtin_themes() {
        let default = Theme::default_dark();
        assert_eq!(default.name, "default");

        let dracula = Theme::dracula();
        assert_eq!(dracula.name, "dracula");

        let gruvbox = Theme::gruvbox();
        assert_eq!(gruvbox.name, "gruvbox");
    }

    #[test]
    fn test_theme_config_resolve_builtin() {
        let config = ThemeConfig {
            name: "dracula".to_string(),
            custom: None,
        };
        let theme = config.resolve();
        assert_eq!(theme.name, "dracula");
    }

    #[test]
    fn test_theme_config_resolve_custom() {
        let custom = Theme::gruvbox();
        let config = ThemeConfig {
            name: "default".to_string(),
            custom: Some(custom.clone()),
        };
        let resolved = config.resolve();
        assert_eq!(resolved.name, "gruvbox");
    }

    #[test]
    fn test_theme_config_default() {
        let config = ThemeConfig::default();
        let theme = config.resolve();
        assert_eq!(theme.name, "default");
    }

    #[test]
    fn test_theme_serialization() {
        let theme = Theme::dracula();
        let toml_str = toml::to_string_pretty(&theme).unwrap();
        assert!(toml_str.contains("name = \"dracula\""));
        // Note: TOML serializes nested structs as [colors.black], [colors.red], etc.
        // so we check for a specific color field instead of [colors]
        assert!(toml_str.contains("[colors.black]"));

        let parsed: Theme = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed, theme);
    }

    #[test]
    fn test_theme_config_serialization() {
        let config = ThemeConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("name = \"default\""));

        let parsed: ThemeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "default");
    }
}
