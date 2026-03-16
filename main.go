package main

import (
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"runtime"
	"strings"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/textinput"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// ansiRegex matches ANSI escape sequences for stripping
var ansiRegex = regexp.MustCompile(`\x1b\[[0-9;]*[a-zA-Z]`)

// stripANSI removes ANSI escape sequences from a string
func stripANSI(s string) string {
	return ansiRegex.ReplaceAllString(s, "")
}

// Warp-inspired dark theme colors
var (
	themeBg       = lipgloss.Color("#1a1b26")
	themeFg       = lipgloss.Color("#c0caf5")
	themeMuted    = lipgloss.Color("#a9b1d6") // Brighter for visibility
	themeAccent   = lipgloss.Color("#7aa2f7")
	themeGreen    = lipgloss.Color("#9ece6a")
	themeYellow   = lipgloss.Color("#e0af68")
	themeRed      = lipgloss.Color("#f7768e")
	themePurple   = lipgloss.Color("#bb9af7")
	themeBorder   = lipgloss.Color("#3b4261")
	themeCmdBlock = lipgloss.Color("#24283b")
	themeInputBg  = lipgloss.Color("#16161e")
)

// init ensures theme variables are used (config.go references them for overrides)
func init() {
	// These variables are used by config.go's ApplyTheme() function
	// Reference them here to satisfy the linter
	_ = themeBg
	_ = themeRed
	_ = themeCmdBlock
	_ = themeInputBg
}

// Styles
var (
	titleStyle = lipgloss.NewStyle().
			Foreground(themeAccent).
			Bold(true).
			Padding(0, 1)

	cmdBlockStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
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
				Border(lipgloss.RoundedBorder()).
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
)

// CommandBlock represents a command + its output
type CommandBlock struct {
	Command string
	Output  string
	IsAI    bool
}

// Model is the main application state
type Model struct {
	viewport    viewport.Model
	textInput   textinput.Model
	spinner     spinner.Model
	blocks      []CommandBlock
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
	showHistory bool
	config      Config
}

// CommandExecMsg is sent when a command finishes executing
type CommandExecMsg struct {
	Command string
	Output  string
	Error   error
}

// InitialModel creates the initial application state
func InitialModel(config Config) Model {
	ti := textinput.New()
	ti.Placeholder = "Type a command or natural language..."
	ti.PlaceholderStyle = lipgloss.NewStyle().Foreground(themeMuted)
	ti.PromptStyle = cmdPromptStyle
	ti.Prompt = "❯ "
	ti.TextStyle = cmdInputStyle
	ti.Focus()

	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = spinnerStyle

	// Load history from file if configured
	history, err := loadHistory(config)
	if err != nil {
		// Log error but continue with empty history
		history = make([]string, 0)
	}

	return Model{
		textInput:   ti,
		spinner:     s,
		blocks:      make([]CommandBlock, 0),
		aiMode:      false,
		aiLoading:   false,
		nlpParser:   NewNLPParser(),
		history:     history,
		maxHistory:  config.MaxHistory,
		showHistory: false,
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
				m.textInput.Prompt = "❯ "
				m.textInput.Placeholder = "Type a command..."
				return m, nil
			}
			return m, tea.Quit

		case tea.KeyEsc:
			// Esc only exits AI mode, never quits
			if m.aiMode {
				m.aiMode = false
				m.textInput.Prompt = "❯ "
				m.textInput.Placeholder = "Type a command..."
			}
			return m, nil

		case tea.KeyTab:
			// Tab toggles AI mode
			m.aiMode = !m.aiMode
			if m.aiMode {
				m.textInput.Prompt = "✨ "
				m.textInput.Placeholder = "Ask AI anything..."
			} else {
				m.textInput.Prompt = "❯ "
				m.textInput.Placeholder = "Type a command..."
			}
			return m, nil

		case tea.KeyEnter:
			input := strings.TrimSpace(m.textInput.Value())
			if input == "" {
				return m, nil
			}

			// Handle /history command
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

			// Add to history (unless it's a duplicate of the last entry)
			if len(m.history) == 0 || m.history[len(m.history)-1] != input {
				m.history = append(m.history, input)
				// Trim history if over max
				if len(m.history) > m.maxHistory {
					m.history = m.history[len(m.history)-m.maxHistory:]
				}
				// Persist to file if configured
						_ = appendToHistory(input, m.config) // Log error but continue - don't interrupt user experience}
					}
			m.showHistory = false

			if m.aiMode {
				// AI mode - stub the call
				m.aiPrompt = input
				m.aiLoading = true
				m.textInput.SetValue("")
				cmds = append(cmds, stubAICall(input))
			} else {
				// Try NLP parsing first
				cmd, matched, desc := m.nlpParser.Parse(input)
				if matched {
					// Execute the translated command
					m.cmdRunning = true
					m.textInput.SetValue("")
					return m, executeCommand(input, cmd, desc)
				} else {
					// Execute raw command
					m.cmdRunning = true
					m.textInput.SetValue("")
					return m, executeCommand(input, input, "")
				}
			}
			return m, tea.Batch(cmds...)

		case tea.KeyPgUp:
			m.viewport.HalfViewUp()
			return m, nil

		case tea.KeyPgDown:
			m.viewport.HalfViewDown()
			return m, nil

		case tea.KeyUp:
			// Scroll up in viewport
			m.viewport.LineUp(1)
			return m, nil

		case tea.KeyDown:
			// Scroll down in viewport
			m.viewport.LineDown(1)
			return m, nil
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

		// Reserve space for input bar (3 lines) and help bar (1 line)
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
		m.blocks = append(m.blocks, CommandBlock{
			Command: m.aiPrompt,
			Output:  msg.Response,
			IsAI:    true,
		})
		m.updateViewport()

	case CommandExecMsg:
		m.cmdRunning = false
		var output string
		if msg.Error != nil {
			output = fmt.Sprintf("[NLP → %s]\nError: %v\n\n%s", msg.Command, msg.Error, msg.Output)
		} else if msg.Output != "" {
			output = msg.Output
		} else {
			output = "(no output)"
		}
		m.blocks = append(m.blocks, CommandBlock{
			Command: msg.Command,
			Output:  output,
			IsAI:    false,
		})
		m.updateViewport()

	case spinner.TickMsg:
		m.spinner, cmd = m.spinner.Update(msg)
		cmds = append(cmds, cmd)

	case tea.MouseMsg:
		// Handle mouse scroll wheel
		switch msg.Button {
		case tea.MouseButtonWheelUp:
			m.viewport.LineUp(3)
		case tea.MouseButtonWheelDown:
			m.viewport.LineDown(3)
		}
	}

	// Update text input
	m.textInput, cmd = m.textInput.Update(msg)
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

// AIResponseMsg is sent when AI responds
type AIResponseMsg struct {
	Response string
}

// stubAICall simulates an AI API call
func stubAICall(prompt string) tea.Cmd {
	return func() tea.Msg {
		// TODO: Wire up actual AI API here
		// For now, return a stubbed response
		response := fmt.Sprintf("AI Response to: %q\n\n[This is a placeholder - wire up your AI API key here]", prompt)
		return AIResponseMsg{Response: response}
	}
}

// executeCommand runs a shell command asynchronously
func (m *Model) updateViewport() {
	if !m.ready {
		return
	}

	// Calculate block width: viewport width minus margins (1 each side) and borders (1 each side)
	// cmdBlockStyle has Margin(0, 1, 1, 1) = left margin 1, and border takes 2 chars (left + right)
	// Also account for padding inside the block Padding(0, 1) = 2 chars
	blockWidth := m.width - 6 // margin(2) + border(2) + padding(2)
	if blockWidth < 10 {
		blockWidth = 10
	}

	var content strings.Builder

	for _, block := range m.blocks {
		var blockContent strings.Builder

		// Command line
		prompt := cmdPromptStyle.Render("❯")
		if block.IsAI {
			prompt = aiIndicatorStyle.Render("✨")
		}
		cmdLine := fmt.Sprintf("%s %s", prompt, cmdInputStyle.Render(block.Command))
		blockContent.WriteString(cmdLine + "\n")

		// Output - strip ANSI codes before adding to prevent width overflow
		if block.Output != "" {
			// Strip ANSI escape sequences to get the visible text width
			cleanOutput := stripANSI(block.Output)
			blockContent.WriteString(outputStyle.Render(cleanOutput))
		}

		// Wrap in styled block with fixed width to ensure consistent border rendering
		styledBlock := cmdBlockStyle.Width(blockWidth).Render(blockContent.String())
		content.WriteString(styledBlock + "\n")
	}

	m.viewport.SetContent(content.String())
	m.viewport.GotoBottom()
}

// updateHistoryView shows the command history
func (m *Model) updateHistoryView() {
	if !m.ready {
		return
	}

	var content strings.Builder
	content.WriteString(titleStyle.Render("📜 Command History") + "\n\n")

	if len(m.history) == 0 {
		content.WriteString(outputStyle.Render("No commands in history yet."))
	} else {
		// Show last 50 commands (or all if less)
		start := 0
		if len(m.history) > 50 {
			start = len(m.history) - 50
		}

		for i := start; i < len(m.history); i++ {
			num := fmt.Sprintf("%4d", i+1)
			content.WriteString(fmt.Sprintf("%s  %s\n", cmdPromptStyle.Render(num), cmdInputStyle.Render(m.history[i])))
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

	// Title bar
	title := titleStyle.Render("Warp Clone • Go + Bubble Tea")
	b.WriteString(title + "\n\n")

	// Viewport (command blocks)
	b.WriteString(m.viewport.View() + "\n")

	// Input bar
	var inputPrompt string
	if m.cmdRunning {
		inputPrompt = fmt.Sprintf("%s Running... %s", m.spinner.View(), m.textInput.View())
	} else if m.aiLoading {
		inputPrompt = fmt.Sprintf("%s Thinking... %s", m.spinner.View(), m.textInput.View())
	} else if m.aiMode {
		inputPrompt = fmt.Sprintf("✨ %s", m.textInput.View())
	} else {
		inputPrompt = m.textInput.View()
	}
	inputBar := inputContainerStyle.Width(m.width - 2).Render(inputPrompt)
	b.WriteString(inputBar + "\n")

	// Help bar
	help := helpStyle.Render("Tab: AI • ↑↓/PgUp/PgDn: Scroll • /history: History • Ctrl+C: Quit")
	b.WriteString(help)

	return b.String()
}

func main() {
	// Setup console for proper Unicode output (fixes border rendering on Windows)
	setupConsole()

	// Load configuration
	config, err := LoadConfig()
	if err != nil {
		fmt.Printf("Warning: Failed to load config, using defaults: %v\n", err)
		config = DefaultConfig()
	}

	// Apply theme colors from config
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

	// Save history on exit if configured
	if model, ok := finalModel.(Model); ok && config.History.PersistToFile {
		_ = saveHistory(model.history, config) // Silent fail - don't interrupt exit}
			}
	}

// executeCommand runs a shell command asynchronously
func executeCommand(originalInput, cmdStr, desc string) tea.Cmd {
	return func() tea.Msg {
		var shell, flag string
		if runtime.GOOS == "windows" {
			shell = "cmd"
			flag = "/c"
		} else {
			shell = "sh"
			flag = "-c"
		}

		cmd := exec.Command(shell, flag, cmdStr)
		cmd.Dir, _ = os.Getwd()

		output, err := cmd.CombinedOutput()

		// Build the output string
		var result string
		if desc != "" {
			result = fmt.Sprintf("[NLP → %s]\n%s\n\n%s", cmdStr, desc, string(output))
		} else {
			result = string(output)
		}

		return CommandExecMsg{
			Command: originalInput,
			Output:  result,
			Error:   err,
		}
	}
}
