# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A free and open-source terminal emulator inspired by Warp, built with Go and the Bubble Tea TUI framework. Features real command execution, natural language parsing, and AI integration (stubbed).

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

# Run tests (when added)
go test ./...
```

## Architecture

Two-file Bubble Tea TUI app following the Elm Architecture pattern.

### Files

- `main.go` - Core TUI application: Model struct, Update loop, View rendering, command execution
- `nlp.go` - Natural language parser: regex-based pattern matching to translate English to shell commands

### Model-View-Update Pattern

- **Model**: Holds all state (viewport, text input, spinner, command blocks, AI mode, command history)
- **Update**: Handles events (key presses, window resize, AI responses, async command completion)
- **View**: Renders UI by composing styled components

### Key Types

- `CommandBlock`: Command + output pair with `IsAI` flag for distinguishing AI responses
- `AIResponseMsg`: Async message from AI API calls
- `CommandExecMsg`: Async message from shell command execution
- `NLPParser`: Pattern-based natural language to command translator

### Message Flow

1. User input → `Update()` handles key events
2. Non-AI commands → `nlpParser.Parse()` tries to translate, falls back to raw input
3. Commands execute via `executeCommand()` (uses `exec.Command` with platform-appropriate shell)
4. Results return as `CommandExecMsg` or `AIResponseMsg` and append to `blocks` slice
5. `updateViewport()` rebuilds scrollable content from blocks

### Styling

Theme colors are `lipgloss.Color` vars at package level (Tokyo Night-inspired). All styles pre-defined as `lipgloss.Style` vars.

### Layout

1. Title bar
2. Viewport (scrollable command blocks)
3. Input bar (prompt changes based on mode: `❯` for commands, `✨` for AI)
4. Help bar

## Keybindings

| Key | Action |
|-----|--------|
| `Tab` | Toggle AI mode |
| `Enter` | Execute command / Submit AI prompt |
| `Esc` | Exit AI mode (never quits app) |
| `Ctrl+C` | Quit app (or exit AI mode when in AI mode) |
| `↑/↓` | Scroll viewport line by line |
| `PgUp/PgDn` | Scroll viewport half-page |
| `/history` | Toggle command history view |

## NLP Parser (`nlp.go`)

The `NLPParser` uses regex patterns with category groupings (navigation, files, system, search, process, network, etc.). Each pattern has a generator function that produces platform-aware commands (Windows vs Unix).

Pattern matching is case-insensitive. If no pattern matches, the raw input passes through to the shell.

## Integration Points

- `stubAICall(prompt string) tea.Cmd` - Currently returns placeholder. Wire up OpenAI/Anthropic/Ollama here.
- NLP patterns - Add new patterns via `p.addPattern()` in `setupPatterns()` with a generator function.

## Tech Stack

- Go 1.21+
- `github.com/charmbracelet/bubbletea` - TUI framework
- `github.com/charmbracelet/lipgloss` - styling
- `github.com/charmbracelet/bubbles/textinput` - text input component
- `github.com/charmbracelet/bubbles/viewport` - scrollable viewport
- `github.com/charmbracelet/bubbles/spinner` - loading spinner

## Note

CONTRIBUTING.md and CI workflows still reference Rust from a previous implementation.
