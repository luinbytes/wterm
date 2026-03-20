package main

// ThemePresets contains predefined theme configurations
var ThemePresets = map[string]ThemeConfig{
	"tokyo-night": {
		Bg:       "#1a1b26",
		Fg:       "#c0caf5",
		Accent:   "#7aa2f7",
		Green:    "#9ece6a",
		Yellow:   "#e0af68",
		Red:      "#f7768e",
		Purple:   "#bb9af7",
		Border:   "#3b4261",
		CmdBlock: "#24283b",
		InputBg:  "#16161e",
	},
	"dracula": {
		Bg:       "#282a36",
		Fg:       "#f8f8f2",
		Accent:   "#bd93f9",
		Green:    "#50fa7b",
		Yellow:   "#f1fa8c",
		Red:      "#ff5555",
		Purple:   "#ff79c6",
		Border:   "#44475a",
		CmdBlock: "#21222c",
		InputBg:  "#1e1e2e",
	},
	"catppuccin-mocha": {
		Bg:       "#1e1e2e",
		Fg:       "#cdd6f4",
		Accent:   "#89b4fa",
		Green:    "#a6e3a1",
		Yellow:   "#f9e2af",
		Red:      "#f38ba8",
		Purple:   "#cba6f7",
		Border:   "#45475a",
		CmdBlock: "#181825",
		InputBg:  "#11111b",
	},
	"gruvbox-dark": {
		Bg:       "#282828",
		Fg:       "#ebdbb2",
		Accent:   "#83a598",
		Green:    "#b8bb26",
		Yellow:   "#fabd2f",
		Red:      "#fb4934",
		Purple:   "#d3869b",
		Border:   "#3c3836",
		CmdBlock: "#1d2021",
		InputBg:  "#1d2021",
	},
	"nord": {
		Bg:       "#2e3440",
		Fg:       "#eceff4",
		Accent:   "#88c0d0",
		Green:    "#a3be8c",
		Yellow:   "#ebcb8b",
		Red:      "#bf616a",
		Purple:   "#b48ead",
		Border:   "#3b4252",
		CmdBlock: "#242933",
		InputBg:  "#242933",
	},
}

// GetThemePreset returns a theme preset by name, or the default if not found
func GetThemePreset(name string) ThemeConfig {
	if preset, ok := ThemePresets[name]; ok {
		return preset
	}
	// Default to tokyo-night
	return ThemePresets["tokyo-night"]
}

// MergeThemeConfigs overlays custom colors on top of a preset
func MergeThemeConfigs(preset, custom ThemeConfig) ThemeConfig {
	merged := preset

	if custom.Bg != "" {
		merged.Bg = custom.Bg
	}
	if custom.Fg != "" {
		merged.Fg = custom.Fg
	}
	if custom.Accent != "" {
		merged.Accent = custom.Accent
	}
	if custom.Green != "" {
		merged.Green = custom.Green
	}
	if custom.Yellow != "" {
		merged.Yellow = custom.Yellow
	}
	if custom.Red != "" {
		merged.Red = custom.Red
	}
	if custom.Purple != "" {
		merged.Purple = custom.Purple
	}
	if custom.Border != "" {
		merged.Border = custom.Border
	}
	if custom.CmdBlock != "" {
		merged.CmdBlock = custom.CmdBlock
	}
	if custom.InputBg != "" {
		merged.InputBg = custom.InputBg
	}

	return merged
}
