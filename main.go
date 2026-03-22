package main

import (
	"fmt"
	"os"
	"os/exec"
	"runtime"
	"strings"
	"syscall"
	"time"

	"github.com/atotto/clipboard"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/textinput"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
)

// Platform-safe prompt symbols to avoid Unicode width issues on Windows
func safePrompt() string {
	if runtime.GOOS == "windows" {
		return ">"
	}
	return "\u276f"
}

func safeAIPrompt() string {
	if runtime.GOOS == "windows" {
		return "*"
	}
	return "\u2728"
}

func safeHistoryIcon() string {
	if runtime.GOOS == "windows" {
		return "#"
	}
	return "\U0001f4dc"
}

// Warp-inspired dark theme colors
var (
	themeBg       = lipgloss.Color("#1a1b26")
	themeFg       = lipgloss.Color("#c0caf5")
	themeMuted    = lipgloss.Color("#a9b1d6")
	themeAccent   = lipgloss.Color("#7aa2f7")
	themeGreen    = lipgloss.Color("#9ece6a")
	themeYellow   = lipgloss.Color("#e0af68")
	themeRed      = lipgloss.Color("#f7768e")
	themePurple   = lipgloss.Color("#bb9af7")
	themeBorder   = lipgloss.Color("#3b4261")
	themeCmdBlock = lipgloss.Color("#24283b")
	themeInputBg  = lipgloss.Color("#16161e")
)

func init() {
	_ = themeBg
	_ = themeRed
	_ = themeCmdBlock
	_ = themeInputBg
}

// Styles -- initialized in initStyles() after setupConsole() runs
var (
	titleStyle          lipgloss.Style
	cmdBlockStyle       lipgloss.Style
	cmdPromptStyle      lipgloss.Style
	cmdInputStyle       lipgloss.Style
	outputStyle         lipgloss.Style
	inputContainerStyle lipgloss.Style
	helpStyle           lipgloss.Style
	aiIndicatorStyle    lipgloss.Style
	spinnerStyle        lipgloss.Style
	exitCodeStyle       lipgloss.Style
	exitCodeErrorStyle  lipgloss.Style
)

// initStyles sets up all lipgloss styles using the correct border for the platform.
// Must be called after setupConsole() so VT mode is active before any rendering.
func initStyles() {
	border := safeBorder()

	titleStyle = lipgloss.NewStyle().
		Foreground(themeAccent).
		Bold(true).
		Padding(0, 1)

	cmdBlockStyle = lipgloss.NewStyle().
		Border(border).
		BorderForeground(themeBorder).
		Padding(0, 1).
		Margin(0, 1, 1, 1)

	cmdPromptStyle = lipgloss.NewStyle().
		Foreground(themeGreen).
		Bold(true)

	cmdInputStyle = lipgloss.NewStyle().
		Foreground(themeFg)

	outputStyle = lipgloss.NewStyle().
		Foreground(themeMuted).
		Padding(0, 1)

	inputContainerStyle = lipgloss.NewStyle().
		Border(border).
		BorderForeground(themeAccent).
		Padding(0, 1)

	helpStyle = lipgloss.NewStyle().
		Foreground(themeMuted).
		Padding(0, 1)

	aiIndicatorStyle = lipgloss.NewStyle().
		Foreground(themePurple).
		Bold(true)

	spinnerStyle = lipgloss.NewStyle().
		Foreground(themeYellow)

	exitCodeStyle = lipgloss.NewStyle().
		Foreground(themeGreen).
		Faint(true)

	exitCodeErrorStyle = lipgloss.NewStyle().
		Foreground(themeRed).
		Faint(true)
}

// suggestionStyle renders auto-suggestion text (grayed out, dim)
var suggestionStyle = lipgloss.NewStyle().Foreground(themeMuted).Faint(true)

// formatExitCode returns a styled exit code indicator with duration
func formatExitCode(exitCode int, duration time.Duration) string {
	if exitCode == -1 {
		return "" // Not applicable (e.g., AI responses)
	}
	durationStr := formatDuration(duration)
	if exitCode == 0 {
		if durationStr != "" {
			return exitCodeStyle.Render(" ✓ " + durationStr)
		}
		return exitCodeStyle.Render(" ✓")
	}
	suffix := fmt.Sprintf(" ✗[%d]", exitCode)
	if durationStr != "" {
		suffix += " " + durationStr
	}
	return exitCodeErrorStyle.Render(suffix)
}

// formatDuration returns a human-friendly duration string for short durations
func formatDuration(d time.Duration) string {
	if d <= 0 {
		return ""
	}
	if d < time.Second {
		return fmt.Sprintf("%.0fms", float64(d.Milliseconds()))
	}
	if d < time.Minute {
		return fmt.Sprintf("%.1fs", d.Seconds())
	}
	return fmt.Sprintf("%.0fm%.0fs", d.Minutes(), d.Seconds()-float64(int(d.Minutes())*60))
}

// CommandBlock represents a command + its output
type CommandBlock struct {
	Command  string
	Output   string
	IsAI     bool
	ExitCode int           // Exit code from shell command (0 = success, -1 = not applicable)
	Duration time.Duration // Command execution duration
}

// Model is the main application state
type Model struct {
	viewport    viewport.Model
	textInput   textinput.Model
	spinner     spinner.Model
	scrollback  *Scrollback
	ready       bool
	width       int
	height      int
	aiMode      bool
	aiLoading   bool
	aiPrompt    string
	nlpParser   *NLPParser
	cmdRunning  bool
	history     []string
	maxHistory  int
	suggestion  string
	showHistory bool
	cwd        string
	config      Config
	// History navigation (arrow key cycling)
	historyNavActive bool
	historyNavIndex  int
	savedInput       string
	// Status message for clipboard operations
	statusMessage string
	statusExpiry  time.Time
}

// CommandExecMsg is sent when a command finishes executing
type CommandExecMsg struct {
	Command  string
	Output   string
	Error    error
	ExitCode int
	Duration time.Duration
}

// InitialModel creates the initial application state
func InitialModel(config Config) Model {
	cwd, _ := os.Getwd()
	ti := textinput.New()
	ti.Placeholder = "Type a command or natural language..."
	ti.PlaceholderStyle = lipgloss.NewStyle().Foreground(themeMuted)
	ti.PromptStyle = cmdPromptStyle
	ti.Prompt = safePrompt() + " "
	ti.TextStyle = cmdInputStyle
	ti.Focus()

	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = spinnerStyle

	history, err := loadHistory(config)
	if err != nil {
		history = make([]string, 0)
	}

	return Model{
		textInput:   ti,
		spinner:     s,
		scrollback:  NewScrollback(config.Scrollback.MaxSize),
		aiMode:      false,
		aiLoading:   false,
		nlpParser:   NewNLPParser(),
		history:     history,
		maxHistory:  config.MaxHistory,
		showHistory: false,
		cwd:        cwd,
		config:      config,
	}
}

// Init initializes the model
func (m Model) Init() tea.Cmd {
	return tea.Batch(
		textinput.Blink,
		m.spinner.Tick,
	)
}

// Update handles events and updates the model
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var (
		cmd  tea.Cmd
		cmds []tea.Cmd
	)

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyCtrlC:
			if m.aiMode {
				m.aiMode = false
				m.textInput.Prompt = safePrompt() + " "
				m.textInput.Placeholder = "Type a command..."
				return m, nil
			}
			if m.cmdRunning && activePTYProcess != nil && activePTYProcess.Process != nil {
				// Send SIGINT to the running PTY process
				//nolint:errcheck
				activePTYProcess.Process.Signal(syscall.SIGINT)
				return m, nil
			}
			return m, tea.Quit
		case tea.KeyEsc:
			if m.aiMode {
				m.aiMode = false
				m.textInput.Prompt = safePrompt() + " "
				m.textInput.Placeholder = "Type a command..."
			}
			return m, nil
		case tea.KeyTab:
			m.aiMode = !m.aiMode
			if m.aiMode {
				m.textInput.Prompt = safeAIPrompt() + " "
				m.textInput.Placeholder = "Ask AI anything..."
			} else {
				m.textInput.Prompt = safePrompt() + " "
				m.textInput.Placeholder = "Type a command..."
			}
			return m, nil
		case tea.KeyEnter:
			input := strings.TrimSpace(m.textInput.Value())
			m.suggestion = ""
			// Reset history navigation on submit
			m.historyNavActive = false
			m.historyNavIndex = 0
			m.savedInput = ""

			if input == "" {
				return m, nil
			}
			if input == "/history" {
				m.showHistory = !m.showHistory
				m.textInput.SetValue("")
				if m.showHistory {
					m.updateHistoryView()
				} else {
					m.updateViewport()
				}
				return m, nil
			}
			// Handle /cd command - change working directory
			if strings.HasPrefix(input, "/cd ") {
				target := strings.TrimSpace(strings.TrimPrefix(input, "/cd "))
				if target == "~" {
					target, _ = os.UserHomeDir()
				}
				if target == "" {
					return m, nil
				}
				if err := os.Chdir(target); err != nil {
					m.scrollback.Append(CommandBlock{Command: input, Output: fmt.Sprintf("cd: %s", err), IsAI: false, ExitCode: -1})
				} else {
					m.cwd, _ = os.Getwd()
					m.scrollback.Append(CommandBlock{Command: input, Output: fmt.Sprintf("Changed directory to %s", m.cwd), IsAI: false, ExitCode: -1})
				}
				m.textInput.SetValue("")
				m.updateViewport()
				return m, nil
			}
			// Handle /search command - filter command history by query
			if strings.HasPrefix(input, "/search ") {
				query := strings.TrimSpace(strings.TrimPrefix(input, "/search "))
				m.scrollback.Append(CommandBlock{Command: input, Output: m.searchHistory(query), IsAI: false, ExitCode: -1})
				m.textInput.SetValue("")
				m.updateViewport()
				return m, nil
			}

			// Handle /clear command
			if input == "/clear" {
				m.scrollback.Clear()
				m.textInput.SetValue("")
				m.updateViewport()
				return m, nil
			}

			// Handle /pwd command - print working directory
			if input == "/pwd" {
				m.scrollback.Append(CommandBlock{Command: input, Output: m.cwd, IsAI: false, ExitCode: -1})
				m.textInput.SetValue("")
				m.updateViewport()
				return m, nil
			}
			if input == "/search" {
				// No query - show usage
				m.scrollback.Append(CommandBlock{Command: "/search", Output: "Usage: /search <query>\nSearchs your command history.\nExample: /search git\n\n" + m.searchHistory(""), IsAI: false, ExitCode: -1})
				m.textInput.SetValue("")
				m.updateViewport()
				return m, nil
			}

			// Add to history (unless it's a duplicate of the last entry)
			if len(m.history) == 0 || m.history[len(m.history)-1] != input {
				m.history = append(m.history, input)
				if len(m.history) > m.maxHistory {
					m.history = m.history[len(m.history)-m.maxHistory:]
				}
				_ = appendToHistory(input, m.config)
			}
			m.showHistory = false
			if m.aiMode {
				m.aiPrompt = input
				m.aiLoading = true
				m.textInput.SetValue("")
				cmds = append(cmds, aiCall(input, m.config))
			} else {
				cmd, matched, desc := m.nlpParser.Parse(input)
				if matched {
					m.cmdRunning = true
					m.textInput.SetValue("")
					return m, executeCommand(input, cmd, desc, m.cwd, m.width, m.height)
				} else {
					m.cmdRunning = true
					m.textInput.SetValue("")
					return m, executeCommand(input, input, "", m.cwd, m.width, m.height)
				}
			}
			return m, tea.Batch(cmds...)

		case tea.KeyRight:
			// Accept auto-suggestion if present
			if m.suggestion != "" {
				m.textInput.SetValue(m.textInput.Value() + m.suggestion)
				m.suggestion = ""
			}
			return m, nil

		case tea.KeyPgUp:
			m.viewport.HalfViewUp()
			return m, nil
		case tea.KeyPgDown:
			m.viewport.HalfViewDown()
			return m, nil
		case tea.KeyUp:
			m.viewport.LineUp(1)
			return m, nil
		case tea.KeyDown:
			m.viewport.LineDown(1)
			return m, nil
		case tea.KeyCtrlUp:
			// Navigate backward through command history
			return m.handleHistoryNavUp()
		case tea.KeyCtrlDown:
			// Navigate forward through command history
			return m.handleHistoryNavDown()
		case tea.KeyCtrlL:
			// Clear the viewport (like a real terminal)
			m.scrollback.Clear()
			m.updateViewport()
			return m, nil
		}

		// Handle Ctrl+Shift+C (copy) and Ctrl+Shift+V (paste)
		// These come through as regular key events with Ctrl modifier in terminals with extended key reporting
		if len(msg.Runes) > 0 {
			switch msg.Runes[0] {
			case 'C':
				// Ctrl+Shift+C: Copy last command output to clipboard
				if m.scrollback.Len() > 0 {
					blocks := m.scrollback.Blocks()
					lastBlock := blocks[len(blocks)-1]
					if lastBlock.Output != "" {
						if err := clipboard.WriteAll(lastBlock.Output); err == nil {
							m.statusMessage = "Copied to clipboard"
							m.statusExpiry = time.Now().Add(2 * time.Second)
						}
					}
				}
				return m, nil
			case 'V':
				// Ctrl+Shift+V: Paste from clipboard into input field
				text, err := clipboard.ReadAll()
				if err == nil && text != "" {
					m.textInput.SetValue(m.textInput.Value() + text)
					m.statusMessage = "Pasted from clipboard"
					m.statusExpiry = time.Now().Add(2 * time.Second)
				}
				return m, nil
			}
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		viewportHeight := m.height - 4
		if viewportHeight < 1 {
			viewportHeight = 1
		}
		m.viewport = viewport.New(m.width, viewportHeight)
		m.viewport.Style = lipgloss.NewStyle().Padding(0, 0)
		m.ready = true
		m.updateViewport()

	case AIResponseMsg:
		m.aiLoading = false
		m.scrollback.Append(CommandBlock{
			Command:  m.aiPrompt,
			Output:   msg.Response,
			IsAI:     true,
			ExitCode: -1, // Not applicable for AI responses
		})
		m.updateViewport()

	case CommandExecMsg:
		m.cmdRunning = false
		var output string
		if msg.Error != nil {
			output = fmt.Sprintf("[NLP -> %s]\nError: %v\n\n%s", msg.Command, msg.Error, msg.Output)
		} else if msg.Output != "" {
			output = msg.Output
		} else {
			output = "(no output)"
		}
		m.scrollback.Append(CommandBlock{
			Command:  msg.Command,
			Output:   output,
			IsAI:     false,
			ExitCode: msg.ExitCode,
			Duration: msg.Duration,
		})
		m.updateViewport()

	case spinner.TickMsg:
		m.spinner, cmd = m.spinner.Update(msg)
		cmds = append(cmds, cmd)

	case tea.MouseMsg:
		switch msg.Button {
		case tea.MouseButtonWheelUp:
			m.viewport.LineUp(3)
		case tea.MouseButtonWheelDown:
			m.viewport.LineDown(3)
		}
	}

	m.textInput, cmd = m.textInput.Update(msg)
	// Update auto-suggestion based on current input
	if !m.cmdRunning && !m.aiLoading && !m.showHistory {
		m.suggestion = m.findSuggestion(m.textInput.Value())
	}
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

// AIResponseMsg is sent when AI responds
type AIResponseMsg struct {
	Response string
}

// searchHistory returns a formatted list of history entries matching query (empty = all)
func (m *Model) searchHistory(query string) string {
	if len(m.history) == 0 {
		return "No commands in history yet."
	}

	var matches []string
	q := strings.ToLower(query)
	for i := len(m.history) - 1; i >= 0; i-- {
		entry := m.history[i]
		if query == "" || strings.Contains(strings.ToLower(entry), q) {
			matches = append(matches, entry)
		}
	}

	if len(matches) == 0 {
		return fmt.Sprintf("No matches found for %q.", query)
	}

	var b strings.Builder
	fmt.Fprintf(&b, "Found %d match(es):\n\n", len(matches))
	for i, entry := range matches {
		fmt.Fprintf(&b, "  %4d  %s\n", i+1, entry)
	}
	return b.String()
}

// handleHistoryNavUp navigates backward through command history
func (m Model) handleHistoryNavUp() (tea.Model, tea.Cmd) {
	if len(m.history) == 0 {
		return m, nil
	}
	// Save current input when first entering history nav
	if !m.historyNavActive {
		m.savedInput = m.textInput.Value()
		m.historyNavActive = true
		m.historyNavIndex = len(m.history) - 1
	} else if m.historyNavIndex > 0 {
		m.historyNavIndex--
	}
	m.textInput.SetValue(m.history[m.historyNavIndex])
	return m, nil
}

// handleHistoryNavDown navigates forward through command history
func (m Model) handleHistoryNavDown() (tea.Model, tea.Cmd) {
	if !m.historyNavActive {
		return m, nil
	}
	if m.historyNavIndex < len(m.history)-1 {
		m.historyNavIndex++
		m.textInput.SetValue(m.history[m.historyNavIndex])
	} else {
		// Past the end - restore saved input
		m.historyNavActive = false
		m.historyNavIndex = 0
		m.textInput.SetValue(m.savedInput)
	}
	return m, nil
}

// updateViewport re-renders all command blocks into the viewport.
// Uses pre-capped strings.Builder to avoid repeated allocations.
func (m *Model) updateViewport() {
	if !m.ready {
		return
	}

	blockWidth := m.width - 6
	if blockWidth < 10 {
		blockWidth = 10
	}

	// Pre-capacity the builder based on estimated content size
	estSize := len(m.scrollback.Blocks()) * (blockWidth * 3)
	content := strings.Builder{}
	content.Grow(estSize)

	for _, block := range m.scrollback.Blocks() {
		var blockContent strings.Builder

		prompt := cmdPromptStyle.Render(safePrompt())
		if block.IsAI {
			prompt = aiIndicatorStyle.Render(safeAIPrompt())
		}
		exitIndicator := ""
		if !block.IsAI {
			exitIndicator = formatExitCode(block.ExitCode, block.Duration)
		}
		cmdLine := fmt.Sprintf("%s %s%s", prompt, cmdInputStyle.Render(block.Command), exitIndicator)
		blockContent.WriteString(cmdLine + "\n")

		if block.Output != "" {
			cleanOutput := ansi.Strip(block.Output)
			blockContent.WriteString(outputStyle.Render(cleanOutput))
		}

		var styledBlock string
		if runtime.GOOS == "windows" {
			sep := strings.Repeat("-", blockWidth)
			styledBlock = sep + "\n" + blockContent.String()
		} else {
			styledBlock = cmdBlockStyle.Width(blockWidth).Render(blockContent.String())
		}
		content.WriteString(styledBlock + "\n")
	}

	m.viewport.SetContent(content.String())
	m.viewport.GotoBottom()
}

// updateHistoryView shows the command history

// findSuggestion returns the most recent history entry that starts with the current input
func (m *Model) findSuggestion(input string) string {
	if input == "" || len(m.history) == 0 {
		return ""
	}
	input = strings.ToLower(input)
	for i := len(m.history) - 1; i >= 0; i-- {
		if strings.HasPrefix(strings.ToLower(m.history[i]), input) {
			candidate := m.history[i]
			if candidate != input {
				return strings.TrimPrefix(candidate, input)
			}
		}
	}
	return ""
}
func (m *Model) updateHistoryView() {
	if !m.ready {
		return
	}

	var content strings.Builder
	content.WriteString(titleStyle.Render(safeHistoryIcon() + " Command History") + "\n\n")

	if len(m.history) == 0 {
		content.WriteString(outputStyle.Render("No commands in history yet."))
	} else {
		start := 0
		if len(m.history) > 50 {
			start = len(m.history) - 50
		}
		for i := start; i < len(m.history); i++ {
			num := fmt.Sprintf("%4d", i+1)
			content.WriteString(fmt.Sprintf("%s %s\n", cmdPromptStyle.Render(num), cmdInputStyle.Render(m.history[i])))
		}
		content.WriteString("\n" + helpStyle.Render(fmt.Sprintf("(%d/%d commands shown)", len(m.history)-start, len(m.history))))
	}

	m.viewport.SetContent(content.String())
	m.viewport.GotoBottom()
}

// View renders the UI
func (m Model) View() string {
	if !m.ready {
		return "\n  Initializing..."
	}

	var b strings.Builder

	title := titleStyle.Render("wterm -- Go + Bubble Tea")
	b.WriteString(title + "\n\n")
	b.WriteString(m.viewport.View() + "\n")

	var inputPrompt string
	if m.cmdRunning {
		inputPrompt = fmt.Sprintf("%s Running... %s", m.spinner.View(), m.textInput.View())
	} else if m.aiLoading {
		inputPrompt = fmt.Sprintf("%s Thinking... %s", m.spinner.View(), m.textInput.View())
	} else if m.aiMode {
		inputPrompt = fmt.Sprintf("%s %s", safeAIPrompt(), m.textInput.View())
	} else {
		inputPrompt = m.textInput.View() + suggestionStyle.Render(m.suggestion)
	}

	inputBar := inputContainerStyle.Width(m.width - 2).Render(inputPrompt)
	b.WriteString(inputBar + "\n")

	// Help bar
	help := helpStyle.Render("Tab: AI • →: Accept suggestion • Ctrl+L: Clear • Ctrl+Up/Down: History • ↑↓/PgUp/PgDn: Scroll • Ctrl+Shift+C: Copy • Ctrl+Shift+V: Paste • /clear: Clear • /history: History • /search: Find • Ctrl+C: Interrupt/Quit")
	b.WriteString(help)

	// Status message (e.g., "Copied to clipboard")
	if m.statusMessage != "" && time.Now().Before(m.statusExpiry) {
		b.WriteString("\n" + helpStyle.Render("  "+m.statusMessage))
	}

	return b.String()
}

func main() {
	// setupConsole MUST run before initStyles so VT mode is active
	setupConsole()
	initStyles()

	config, err := LoadConfig()
	if err != nil {
		fmt.Printf("Warning: Failed to load config, using defaults: %v\n", err)
		config = DefaultConfig()
	}
	config.ApplyTheme()

	initialModel := InitialModel(config)

	p := tea.NewProgram(
		initialModel,
		tea.WithAltScreen(),
		tea.WithMouseCellMotion(),
	)

	finalModel, err := p.Run()
	if err != nil {
		fmt.Printf("Error: %v", err)
		os.Exit(1)
	}

	if model, ok := finalModel.(Model); ok && config.History.PersistToFile {
		_ = saveHistory(model.history, config)
	}
}

// executeCommand runs a shell command asynchronously using PTY for better shell integration
func executeCommand(originalInput, cmdStr, desc, cwd string, termWidth, termHeight int) tea.Cmd {
	return func() tea.Msg {
		start := time.Now()

		var shell, flag string
		if runtime.GOOS == "windows" {
			shell = "cmd"
			flag = "/c"
		} else {
			shell = "sh"
			flag = "-c"
		}

		output, err := PTYCommand(shell, flag, cmdStr, cwd, termWidth, termHeight)
		duration := time.Since(start)

		// Extract exit code
		exitCode := 0
		if err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				exitCode = exitErr.ExitCode()
			} else {
				exitCode = 1 // Generic error
			}
		}

		var result string
		if desc != "" {
			result = fmt.Sprintf("[NLP -> %s]\n%s\n\n%s", cmdStr, desc, string(output))
		} else {
			result = string(output)
		}

		return CommandExecMsg{
			Command:  originalInput,
			Output:   result,
			Error:    err,
			ExitCode: exitCode,
			Duration: duration,
		}
	}
}
