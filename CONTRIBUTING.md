# Contributing to wterm

Thank you for considering contributing to wterm! Contributions are what make open source great.

## Development Setup

### Prerequisites

- **Go 1.21+** — [Install Go](https://go.dev/dl/)
- **git** — for version control

### Getting Started

```bash
# Clone the repo
git clone https://github.com/luinbytes/wterm.git
cd wterm

# Build
go build -o wterm .

# Run
go run .

# Run tests
go test ./...

# Format code
go fmt ./...
```

## Development Workflow

1. **Fork** the repo on GitHub
2. **Clone** your fork locally
   ```bash
   git clone https://github.com/your-username/wterm.git
   cd wterm
   ```
3. **Create a branch** for your work
   ```bash
   # Use conventional prefixes
   git checkout -b feat/my-feature    # new feature
   git checkout -b fix/bug-description # bug fix
   git checkout -b docs/update-x       # documentation
   ```
4. **Make changes**, write tests, verify they pass
5. **Push** to your fork
   ```bash
   git push origin feat/my-feature
   ```
6. **Open a Pull Request** against `main`

## Coding Standards

- Run `go fmt ./...` before committing
- Run `go vet ./...` and fix all warnings
- Write tests for new functionality (`go test ./...`)
- Document exported functions and types with godoc comments
- Follow [Effective Go](https://go.dev/doc/effective_go)
- Keep changes focused — one PR, one purpose

## Commit Guidelines

- Use conventional commit prefixes: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
- Present tense, imperative mood: `feat: add scrollback buffer` not `added scrollback buffer`
- Keep the first line under 72 characters
- Reference issues in the body: `Closes #42` or `Fixes #42`

## Testing

- Run `go test ./...` before pushing
- Add tests for new features and bug fixes
- For TUI changes, test via tmux:

```bash
tmux new-session -d -s wterm-test
tmux send-keys -t wterm-test "./wterm" Enter
sleep 1
tmux send-keys -t wterm-test "echo hello" Enter
sleep 0.5
tmux capture-pane -t wterm-test -p | tail -20
tmux kill-session -t wterm-test
```

## Pull Request Process

1. Update documentation (CLAUDE.md, README.md) if adding new features or changing architecture
2. Ensure all tests pass: `go test ./...`
3. Keep PRs focused on a single change
4. Use a descriptive PR title matching the commit message

## Project Structure

See [CLAUDE.md](CLAUDE.md) for detailed architecture documentation.

## Questions?

Open an issue with the "question" label or reach out to the maintainers.

---

Thank you for contributing!
