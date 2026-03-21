package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"gopkg.in/yaml.v3"
)

// ScrollbackConfig holds scrollback buffer configuration
type ScrollbackConfig struct {
	MaxSize int `yaml:"maxSize"` // max command blocks to retain (default 1000)
}

// Config holds application configuration
type Config struct {
	MaxHistory int              `yaml:"maxHistory"`
	APIKey     string           `yaml:"apiKey"`
	Provider   string           `yaml:"provider"` // "openai", "anthropic", "ollama", etc.
	Theme      ThemeConfig      `yaml:"theme,omitempty"`
	History    HistoryConfig    `yaml:"history,omitempty"`
	Scrollback ScrollbackConfig `yaml:"scrollback,omitempty"`
}

// ThemeConfig holds theme color overrides
type ThemeConfig struct {
	Name     string `yaml:"name,omitempty"` // preset name: tokyo-night, dracula, catppuccin-mocha, gruvbox-dark, nord
	Bg       string `yaml:"bg,omitempty"`
	Fg       string `yaml:"fg,omitempty"`
	Accent   string `yaml:"accent,omitempty"`
	Green    string `yaml:"green,omitempty"`
	Yellow   string `yaml:"yellow,omitempty"`
	Red      string `yaml:"red,omitempty"`
	Purple   string `yaml:"purple,omitempty"`
	Border   string `yaml:"border,omitempty"`
	CmdBlock string `yaml:"cmdBlock,omitempty"`
	InputBg  string `yaml:"inputBg,omitempty"`
}

// HistoryConfig holds history-related configuration
type HistoryConfig struct {
	PersistToFile bool   `yaml:"persistToFile"`
	Path          string `yaml:"path"`
	MaxFileSizeKB int    `yaml:"maxFileSizeKB"`
}

// DefaultConfig returns a default configuration
func DefaultConfig() Config {
	homeDir, _ := os.UserHomeDir()
	historyPath := filepath.Join(homeDir, ".wterm", "history.txt")

	return Config{
		MaxHistory: 1000,
		APIKey:     "",
		Provider:   "",
		Theme: ThemeConfig{
			Name: "tokyo-night",
		},
		History: HistoryConfig{
			PersistToFile: false,
			Path:          historyPath,
			MaxFileSizeKB: 100,
		},
		Scrollback: ScrollbackConfig{
			MaxSize: 1000,
		},
	}
}

// GetConfigPath returns the path to the config file
func GetConfigPath() (string, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}

	configDir := filepath.Join(homeDir, ".wterm")
	configPath := filepath.Join(configDir, "config.yaml")

	return configPath, nil
}

// LoadConfig loads the configuration from file or creates a default one
func LoadConfig() (Config, error) {
	configPath, err := GetConfigPath()
	if err != nil {
		return DefaultConfig(), fmt.Errorf("failed to get config path: %w", err)
	}

	// Check if config file exists
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		// Create default config
		defaultConfig := DefaultConfig()

		// Ensure config directory exists
		configDir := filepath.Dir(configPath)
		if err := os.MkdirAll(configDir, 0755); err != nil {
			return defaultConfig, fmt.Errorf("failed to create config directory: %w", err)
		}

		// Write default config
		data, err := yaml.Marshal(defaultConfig)
		if err != nil {
			return defaultConfig, fmt.Errorf("failed to marshal default config: %w", err)
		}

		if err := os.WriteFile(configPath, data, 0644); err != nil {
			return defaultConfig, fmt.Errorf("failed to write default config: %w", err)
		}

		return defaultConfig, nil
	}

	// Read existing config
	data, err := os.ReadFile(configPath)
	if err != nil {
		return DefaultConfig(), fmt.Errorf("failed to read config file: %w", err)
	}

	// Parse YAML
	var config Config
	if err := yaml.Unmarshal(data, &config); err != nil {
		return DefaultConfig(), fmt.Errorf("failed to parse config file: %w", err)
	}

	// Validate and apply defaults
	if config.MaxHistory <= 0 {
		config.MaxHistory = 1000
	}

	if config.Provider != "" {
		config.Provider = strings.ToLower(strings.TrimSpace(config.Provider))
	}

	if config.History.Path == "" {
		homeDir, _ := os.UserHomeDir()
		config.History.Path = filepath.Join(homeDir, ".wterm", "history.txt")
	}

	if config.History.MaxFileSizeKB <= 0 {
		config.History.MaxFileSizeKB = 100
	}

	// Validate scrollback config
	if config.Scrollback.MaxSize <= 0 {
		config.Scrollback.MaxSize = 1000
	}

	// Apply theme preset with custom color overlay
	config.Theme = resolveTheme(config.Theme)

	return config, nil
}

// resolveTheme merges a preset with custom color overrides
func resolveTheme(theme ThemeConfig) ThemeConfig {
	// Default to tokyo-night if no name specified
	themeName := theme.Name
	if themeName == "" {
		themeName = "tokyo-night"
	}

	// Get the preset
	preset := GetThemePreset(themeName)

	// Save custom overrides
	custom := ThemeConfig{
		Bg:       theme.Bg,
		Fg:       theme.Fg,
		Accent:   theme.Accent,
		Green:    theme.Green,
		Yellow:   theme.Yellow,
		Red:      theme.Red,
		Purple:   theme.Purple,
		Border:   theme.Border,
		CmdBlock: theme.CmdBlock,
		InputBg:  theme.InputBg,
	}

	// Merge preset with custom overlays
	merged := MergeThemeConfigs(preset, custom)
	merged.Name = themeName // preserve the name

	return merged
}

// ApplyTheme applies theme colors from config to global variables
func (c *Config) ApplyTheme() {
	// Only override if theme colors are specified in config
	if c.Theme.Bg != "" {
		themeBg = parseColor(c.Theme.Bg)
	}
	if c.Theme.Fg != "" {
		themeFg = parseColor(c.Theme.Fg)
	}
	if c.Theme.Accent != "" {
		themeAccent = parseColor(c.Theme.Accent)
	}
	if c.Theme.Green != "" {
		themeGreen = parseColor(c.Theme.Green)
	}
	if c.Theme.Yellow != "" {
		themeYellow = parseColor(c.Theme.Yellow)
	}
	if c.Theme.Red != "" {
		themeRed = parseColor(c.Theme.Red)
	}
	if c.Theme.Purple != "" {
		themePurple = parseColor(c.Theme.Purple)
	}
	if c.Theme.Border != "" {
		themeBorder = parseColor(c.Theme.Border)
	}
	if c.Theme.CmdBlock != "" {
		themeCmdBlock = parseColor(c.Theme.CmdBlock)
	}
	if c.Theme.InputBg != "" {
		themeInputBg = parseColor(c.Theme.InputBg)
	}
}

// parseColor converts a hex color string to lipgloss.Color
func parseColor(hex string) lipgloss.Color {
	// Ensure it's a valid hex color format
	if strings.HasPrefix(hex, "#") {
		return lipgloss.Color(hex)
	}
	// If no prefix, add it
	if len(hex) == 6 || len(hex) == 3 {
		return lipgloss.Color("#" + hex)
	}
	// Invalid format, return default
	return themeFg
}
