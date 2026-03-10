# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A free and open-source terminal emulator inspired by Warp, built with Go and the Bubble Tea TUI framework. The project is in early development with basic TUI working and AI integration stubs ready for implementation.

## Commands

```bash
# Run the application
go run .

# Build a binary
go build -o warp-clone .

# Format code
go fmt ./...

# Run linter (install first: go install golang.org/x/tools/cmd/goimports@latest)
goimports -w .

# Run tests (when added)
go test ./...
```

## Architecture

The entire application lives in `main.go` as a single-file Bubble Tea TUI app. Key components:

### Model-View-Update Pattern (Bubble Tea)

- **Model**: `Model` struct holds all application state (viewport, text input, spinner, command blocks, AI mode flags)
- **Update**: `Update()` method handles all events (key presses, window resize, AI responses, spinner ticks)
- **View**: `View()` method renders the UI by composing styled components

### Key Types

- `CommandBlock`: Represents a command + its output, with an `IsAI` flag for AI responses
- `AIResponseMsg`: Message type for async AI responses

### Styling

Theme colors are defined as `lipgloss.Color` variables at package level (Tokyo Night-inspired). All styles are pre-defined as `lipgloss.Style` variables.

### Stub Functions (Key Integration Points)

- `stubAICall(prompt string) tea.Cmd`: Returns a command that produces `AIResponseMsg` - wire up AI providers here
- `stubCommand(cmd string) string`: Returns simulated output - wire up PTY/shell execution here

Both are package-level functions, not methods on Model.

### Layout Structure

The UI is divided into:
1. Title bar
2. Viewport (scrollable command blocks)
3. Input bar (changes based on mode)
4. Help bar (keybindings)

## Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+Space` | Toggle AI mode |
| `Enter` | Execute command / Submit AI prompt |
| `Esc` | Exit AI mode (never quits) |
| `Ctrl+C` | Quit app (or exit AI mode when in AI mode) |

## Tech Stack

- Go 1.22+
- `github.com/charmbracelet/bubbletea` - TUI framework
- `github.com/charmbracelet/lipgloss` - styling
- `github.com/charmbracelet/bubbles/textinput` - text input component
- `github.com/charmbracelet/bubbles/viewport` - scrollable viewport
- `github.com/charmbracelet/bubbles/spinner` - loading spinner

## Note

CONTRIBUTING.md and CI workflows still reference Rust from a previous implementation. These need updating to reflect the current Go codebase.
