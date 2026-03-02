//! Application settings loaded from config file

use crate::config::keybindings::Keybindings;
use crate::config::theme::ThemeConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Terminal settings
    #[serde(default)]
    pub terminal: TerminalConfig,
    /// Font settings
    #[serde(default)]
    pub font: FontConfig,
    /// Theme settings
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Keybindings
    #[serde(default)]
    pub keybindings: Keybindings,
    /// Window settings
    #[serde(default)]
    pub window: WindowConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            terminal: TerminalConfig::default(),
            font: FontConfig::default(),
            theme: ThemeConfig::default(),
            keybindings: Keybindings::default(),
            window: WindowConfig::default(),
        }
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// Shell command to spawn (defaults to $SHELL or /bin/sh)
    #[serde(default)]
    pub shell: Option<String>,
    /// Initial terminal width in columns
    #[serde(default = "default_cols")]
    pub cols: u16,
    /// Initial terminal height in rows
    #[serde(default = "default_rows")]
    pub rows: u16,
    /// Working directory for the shell (empty means current directory)
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Environment variables to set
    #[serde(default)]
    pub env: Vec<(String, String)>,
}

fn default_cols() -> u16 {
    120
}

fn default_rows() -> u16 {
    40
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            shell: None,
            cols: default_cols(),
            rows: default_rows(),
            working_dir: None,
            env: Vec::new(),
        }
    }
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Font family name
    #[serde(default = "default_font_family")]
    pub family: String,
    /// Font size in points
    #[serde(default = "default_font_size")]
    pub size: f32,
    /// Line height multiplier (1.0 = normal, 1.2 = 20% extra space)
    #[serde(default = "default_line_height")]
    pub line_height: f32,
}

fn default_font_family() -> String {
    "monospace".to_string()
}

fn default_font_size() -> f32 {
    14.0
}

fn default_line_height() -> f32 {
    1.2
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size: default_font_size(),
            line_height: default_line_height(),
        }
    }
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Horizontal and vertical padding in pixels
    #[serde(default = "default_padding")]
    pub padding: (u16, u16),
    /// Window opacity/transparency (0.0 = fully transparent, 1.0 = opaque)
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

fn default_padding() -> (u16, u16) {
    (10, 10)
}

fn default_opacity() -> f32 {
    1.0
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            padding: default_padding(),
            opacity: default_opacity(),
        }
    }
}

impl Config {
    /// Load configuration from the default location
    ///
    /// Looks for config at:
    /// 1. ~/.config/warp-foss/config.toml
    /// 2. Creates default config if not found
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            Self::load_from_path(&config_path)
        } else {
            // Create default config
            let config = Self::default();
            config.save_to_path(&config_path)?;
            tracing::info!("Created default config at {:?}", config_path);
            Ok(config)
        }
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", path))
    }

    /// Save configuration to a specific path
    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(path, &contents)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        Ok(())
    }

    /// Get the default config file path
    ///
    /// Priority:
    /// 1. $XDG_CONFIG_HOME/warp-foss/config.toml
    /// 2. ~/.config/warp-foss/config.toml
    pub fn config_path() -> Result<PathBuf> {
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            Ok(PathBuf::from(xdg_config).join("warp-foss").join("config.toml"))
        } else {
            let home = dirs::home_dir()
                .context("Could not determine home directory")?;
            Ok(home.join(".config").join("warp-foss").join("config.toml"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.terminal.cols, 120);
        assert_eq!(config.terminal.rows, 40);
        assert_eq!(config.font.family, "monospace");
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.font.line_height, 1.2);
        assert_eq!(config.window.padding, (10, 10));
        assert_eq!(config.window.opacity, 1.0);
        assert_eq!(config.theme.name, "default");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[terminal]"));
        assert!(toml_str.contains("[font]"));
        assert!(toml_str.contains("[theme]"));
        assert!(toml_str.contains("[window]"));

        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.terminal.cols, config.terminal.cols);
        assert_eq!(parsed.font.size, config.font.size);
        assert_eq!(parsed.font.line_height, config.font.line_height);
    }

    #[test]
    fn test_custom_config() {
        let toml_str = r#"
[terminal]
shell = "/bin/fish"
cols = 100
rows = 30
working_dir = "/home/user"

[font]
family = "Fira Code"
size = 16.0
line_height = 1.4

[theme]
name = "dracula"

[window]
padding = [20, 15]
opacity = 0.95
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.terminal.shell, Some("/bin/fish".to_string()));
        assert_eq!(config.terminal.cols, 100);
        assert_eq!(config.terminal.rows, 30);
        assert_eq!(config.terminal.working_dir, Some("/home/user".to_string()));
        assert_eq!(config.font.family, "Fira Code");
        assert_eq!(config.font.size, 16.0);
        assert_eq!(config.font.line_height, 1.4);
        assert_eq!(config.theme.name, "dracula");
        assert_eq!(config.window.padding, (20, 15));
        assert!(f32::abs(config.window.opacity - 0.95) < 0.001);
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
[font]
size = 18.0
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        // Terminal should use defaults
        assert_eq!(config.terminal.cols, 120);
        assert_eq!(config.terminal.rows, 40);
        // Font should use custom value but default family and line_height
        assert_eq!(config.font.family, "monospace");
        assert_eq!(config.font.size, 18.0);
        assert_eq!(config.font.line_height, 1.2);
        // Window should use defaults
        assert_eq!(config.window.padding, (10, 10));
        assert_eq!(config.window.opacity, 1.0);
        // Theme should use defaults
        assert_eq!(config.theme.name, "default");
    }

    #[test]
    fn test_font_config_defaults() {
        let font = FontConfig::default();
        assert_eq!(font.family, "monospace");
        assert_eq!(font.size, 14.0);
        assert_eq!(font.line_height, 1.2);
    }

    #[test]
    fn test_window_config_defaults() {
        let window = WindowConfig::default();
        assert_eq!(window.padding, (10, 10));
        assert_eq!(window.opacity, 1.0);
    }
}
