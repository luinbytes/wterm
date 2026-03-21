# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

wterm is a free and open-source terminal emulator built with Go and the Bubble Tea TUI framework. Features PTY-based command execution, natural language parsing, theme presets, clipboard support, exit code tracking, and AI integration (stubbed).

## Commands

```bash
# Run the application
go run .

# Build a binary
go build -o wterm .

# Format code
go fmt ./...

# Run linter (install first: go install golang.org/x/tools/cmd/goimports@latest)
goimports -w .

# Run tests
go test ./...
```

## Architecture

Bubble Tea TUI app following the Elm Architecture (Model-View-Update). Commands execute via pseudo-terminals (PTY), not exec.Command.

### Files

| File | Purpose |
|------|---------|
| `main.go` | Core TUI: Model struct, Update loop, View rendering, command dispatch, keybindings |
| `nlp.go` | Natural language parser: regex-based pattern matching to translate English to shell commands |
| `nlp_test.go` | Unit tests for NLP parser (69+ test cases across all pattern categories) |
| `pty.go` | PTY integration: `PTYCommand()` executes shell commands in a pseudo-terminal via `creack/pty` |
| `config.go` | Configuration: YAML config loading/saving, theme resolution, scrollback settings |
| `themes.go` | Theme presets: 5 built-in themes (tokyo-night, dracula, catppuccin-mocha, gruvbox-dark, nord) |
| `history.go` | Command history: file-based persistence, truncation, append |
| `scrollback.go` | Scrollback buffer: bounded ring buffer for command output with configurable size and thread safety |
| `scrollback_test.go` | Unit tests for scrollback buffer |
| `borders.go` | Platform-safe border rendering: RoundedBorder on Unix, HiddenBorder on Windows |
| `console_other.go` | Console setup no-op for Unix |
| `console_windows.go` | Windows console setup: UTF-8 code page, VT processing for ANSI/box-drawing |

### Model-View-Update Pattern

- **Model**: Holds all state (viewport, text input, spinner, scrollback buffer, AI mode, command history, config)
- **Update**: Handles events (key presses, window resize, AI responses, async command completion)
- **View**: Renders UI by composing styled components

### Key Types

- `CommandBlock`: Command + output pair with `IsAI` flag and `ExitCode` for distinguishing command types
- `Scrollback`: Bounded ring buffer for command output history (thread-safe, configurable max size)
- `AIResponseMsg`: Async message from AI API calls
- `CommandExecMsg`: Async message from shell command execution
- `NLPParser`: Pattern-based natural language to command translator
- `Config`: Application configuration (history, theme, scrollback, API keys)

### Message Flow

1. User input → `Update()` handles key events
2. Non-AI commands → `nlpParser.Parse()` tries to translate, falls back to raw input
3. Commands execute via `executeCommand()` → `PTYCommand()` (pseudo-terminal with dynamic sizing)
4. Results return as `CommandExecMsg` or `AIResponseMsg` and append to `scrollback` buffer
5. `updateViewport()` rebuilds scrollable content from scrollback blocks

### Built-in Commands

| Command | Description |
|---------|-------------|
| `/cd <path>` | Change working directory |
| `/pwd` | Print working directory |
| `/clear` or `Ctrl+L` | Clear all command blocks |
| `/history` | Toggle command history view |
| `/search <query>` | Search command history |

### Styling

Theme colors are `lipgloss.Color` vars at package level. 5 built-in presets via `themes.go`. Custom color overrides supported via `config.yaml`. Config resolves: preset → custom overlay → `ApplyTheme()`.

### Layout

1. Title bar ("wterm -- Go + Bubble Tea")
2. Viewport (scrollable command blocks via Bubble Tea viewport component)
3. Input bar (prompt: `❯` for commands, `✨` for AI, with auto-suggestion)
4. Help bar + status messages

## Keybindings

| Key | Action |
|-----|--------|
| `Tab` | Toggle AI mode |
| `Enter` | Execute command / Submit AI prompt |
| `Esc` | Exit AI mode |
| `Ctrl+C` | Quit app (or exit AI mode when in AI mode) |
| `Ctrl+L` | Clear all blocks |
| `↑/↓` | Scroll viewport line by line |
| `PgUp/PgDn` | Scroll viewport half-page |
| `Ctrl+Up/Down` | Navigate command history (input cycling) |
| `→` | Accept auto-suggestion |
| `Ctrl+Shift+C` | Copy last command output to clipboard |
| `Ctrl+Shift+V` | Paste from clipboard into input |

## NLP Parser (`nlp.go`)

The `NLPParser` uses regex patterns with category groupings: navigation, files, system, search, process, network, text/file operations, environment variables, and more. Each pattern has a generator function that produces platform-aware commands (Windows vs Unix).

Pattern matching is case-insensitive. If no pattern matches, the raw input passes through to the shell.

## Integration Points

- `stubAICall(prompt string) tea.Cmd` — Currently returns placeholder. Wire up OpenAI/Anthropic/Ollama here. Config has `apiKey` and `provider` fields ready.
- NLP patterns — Add new patterns via `p.addPattern()` in `setupPatterns()` with a generator function.
- Themes — Add new presets to `ThemePresets` map in `themes.go`.
- Config — Add new fields to `Config` struct in `config.go`, update `DefaultConfig()` and validation in `LoadConfig()`.

## Configuration

Config file: `~/.wterm/config.yaml` (auto-created with defaults on first run).

```yaml
maxHistory: 1000
apiKey: ""
provider: ""
theme:
  name: tokyo-night  # or: dracula, catppuccin-mocha, gruvbox-dark, nord
  # Custom color overrides:
  # bg: "#1a1b26"
  # fg: "#c0caf5"
history:
  persistToFile: false
  path: ~/.wterm/history.txt
  maxFileSizeKB: 100
scrollback:
  maxSize: 1000
```

## Tech Stack

- Go 1.21+
- `github.com/charmbracelet/bubbletea` — TUI framework
- `github.com/charmbracelet/lipgloss` — styling
- `github.com/charmbracelet/bubbles/textinput` — text input component
- `github.com/charmbracelet/bubbles/viewport` — scrollable viewport
- `github.com/charmbracelet/bubbles/spinner` — loading spinner
- `github.com/charmbracelet/x/ansi` — ANSI escape sequence handling (ECMA-48)
- `github.com/creack/pty` — pseudo-terminal for real shell execution
- `golang.org/x/term` — dynamic terminal size detection
- `github.com/atotto/clipboard` — cross-platform clipboard (Ctrl+Shift+C/V)
- `gopkg.in/yaml.v3` — YAML config parsing
