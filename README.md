# Warp FOSS Clone

A free and open-source terminal emulator inspired by [Warp](https://warp.dev/), built with Go and Bubble Tea TUI.

> ⚡ **Status:** Early development. Basic TUI works with AI integration stubs.

## Features

- 🎨 **Warp-inspired dark theme** with Lip Gloss styling
- 🖥️ **Bubble Tea TUI** - clean, responsive terminal interface
- 🤖 **AI integration** - BYOK, stubbed and ready for your API key
- 📦 **Command block grouping** - each command + output in styled blocks
- 📝 **Command history** - persistent history with configurable storage
- ⚡ **Fast, written in Go** - single binary, no runtime dependencies

## Stack

- **Go 1.21+**
- [github.com/charmbracelet/bubbletea](https://github.com/charmbracelet/bubbletea) — TUI framework
- [github.com/charmbracelet/lipgloss](https://github.com/charmbracelet/lipgloss) — styling/theming
- [github.com/charmbracelet/bubbles](https://github.com/charmbracelet/bubbles) — textinput, viewport, spinner

## Quick Start

```bash
# Clone the repo
git clone https://github.com/luinbytes/warp-foss-clone.git
cd warp-foss-clone

# Run
go run .

# Build
go build -o warp-clone .
```

## Keybindings

| Keybinding | Action |
|------------|--------|
| `Tab` | Toggle AI mode |
| `Enter` | Execute command / Submit AI prompt |
| `Esc` | Exit AI mode |
| `Ctrl+C` | Quit (or exit AI mode) |
| `↑/↓` | Scroll viewport line by line |
| `PgUp/PgDn` | Scroll viewport half-page |
| `/history` | Toggle command history view |

## Configuration

The application creates a configuration file at `~/.warp-clone/config.yaml` on first run. You can customize:

- **Theme colors** - Override any color from the Tokyo Night theme
- **History persistence** - Enable/disable and configure history file location
- **History size** - Maximum number of commands to remember (default: 1000)
- **History file size** - Maximum size of history file in KB (default: 100)

Example configuration:

```yaml
maxHistory: 1000
apiKey: ""
provider: ""

history:
  persistToFile: true
  path: "~/.warp-clone/history.txt"
  maxFileSizeKB: 100

theme:
  bg: "#1a1b26"
  fg: "#c0caf5"
  accent: "#7aa2f7"
  # ... and more
```

## AI Integration

AI integration is stubbed and ready to wire up. Look for `stubAICall` in `main.go`:

```go
func (m *Model) stubAICall(prompt string) tea.Cmd {
    return func() tea.Msg {
        // TODO: Wire up actual AI API here
        // Example: OpenAI, Anthropic, Ollama, etc.
        response := fmt.Sprintf("AI Response to: %q", prompt)
        return AIResponseMsg{Response: response}
    }
}
```

### Supported Providers (TODO)

- [ ] OpenAI
- [ ] Anthropic Claude
- [ ] Ollama (local)

## Terminal Execution

Commands are executed via `exec.Command` using the platform-appropriate shell (`sh` on Unix, `cmd` on Windows). The output is captured and displayed in styled command blocks.

## Project Structure

```
warp-foss-clone/
├── main.go          # Entry point + Bubble Tea app
├── config.go        # Configuration management
├── history.go       # History persistence
├── nlp.go           # Natural language parser
├── console_*.go     # Platform-specific console setup
├── go.mod           # Go module with dependencies
├── README.md        # This file
├── CONTRIBUTING.md  # Contribution guidelines
└── .github/         # GitHub templates
```

## Roadmap

- [ ] Real shell/PTY integration
- [ ] Wire up AI API (configurable provider)
- [ ] Scrollback buffer
- [ ] Tab completion
- [x] Command history
- [ ] Split panes
- [ ] Tab management
- [x] Configuration file (~/.warp-clone/config.yaml)
- [x] Theme customization

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- Inspired by [Warp](https://warp.dev/)
- Built with [Bubble Tea](https://github.com/charmbracelet/bubbletea) by [Charm](https://charm.sh/)
- Theme based on [Tokyo Night](https://github.com/enkia/tokyo-night-vscode-theme)
