package main

import (
  "fmt"
  "strings"

  "github.com/charmbracelet/bubbles/spinner"
  "github.com/charmbracelet/bubbles/textinput"
  "github.com/charmbracelet/bubbles/viewport"
  "github.com/charmbracelet/bubbletea"
  "github.com/charmbracelet/lipgloss"
)

// Warp-inspired dark theme colors
var (
  themeBg         = lipgloss.Color("#1a1b26")
  themeFg         = lipgloss.Color("#c0caf5")
  themeMuted      = lipgloss.Color("#565f89")
  themeAccent     = lipgloss.Color("#7aa2f7")
  themeGreen      = lipgloss.Color("#9ece6a")
  themeYellow     = lipgloss.Color("#e0af68")
  themeRed        = lipgloss.Color("#f7768e")
  themePurple     = lipgloss.Color("#bb9af7")
  themeBorder     = lipgloss.Color("#3b4261")
  themeCmdBlock   = lipgloss.Color("#24283b")
  themeInputBg    = lipgloss.Color("#16161e")
)

// Styles
var (
  titleStyle = lipgloss.NewStyle().
      Foreground(themeAccent).
      Bold(true).
      Padding(0, 1)

  cmdBlockStyle = lipgloss.NewStyle().
      Border(lipgloss.RoundedBorder()).
      BorderForeground(themeBorder).
      Background(themeCmdBlock).
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
      Background(themeInputBg).
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
}

// InitialModel creates the initial application state
func InitialModel() Model {
  ti := textinput.New()
  ti.Placeholder = "Type a command..."
  ti.PlaceholderStyle = lipgloss.NewStyle().Foreground(themeMuted)
  ti.PromptStyle = cmdPromptStyle
  ti.Prompt = "❯ "
  ti.TextStyle = cmdInputStyle
  ti.Focus()

  s := spinner.New()
  s.Spinner = spinner.Dot
  s.Style = spinnerStyle

  return Model{
    textInput: ti,
    spinner:   s,
    blocks:    make([]CommandBlock, 0),
    aiMode:    false,
    aiLoading: false,
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
func (m *Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
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

    case tea.KeyCtrlSpace:
      // Toggle AI mode
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

      if m.aiMode {
        // AI mode - stubbed call
        m.aiPrompt = input
        m.aiLoading = true
        m.textInput.SetValue("")
        cmds = append(cmds, stubAICall(input))
      } else {
        // Regular command - stub execution
        output := stubCommand(input)
        m.blocks = append(m.blocks, CommandBlock{
          Command: input,
          Output:  output,
          IsAI:    false,
        })
        m.textInput.SetValue("")
        m.updateViewport()
      }
      return m, tea.Batch(cmds...)

    case tea.WindowSizeMsg:
      m.width = msg.Width
      m.height = msg.Height

      // Reserve space for input bar (3 lines) and help bar (1 line)
      viewportHeight := m.height - 4
      if viewportHeight < 1 {
        viewportHeight = 1
      }

      m.viewport = viewport.New(m.width, viewportHeight)
      m.viewport.Style = lipgloss.NewStyle().
        Background(themeBg).
        Padding(0, 0)
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

    case spinner.TickMsg:
      m.spinner, cmd = m.spinner.Update(msg)
      cmds = append(cmds, cmd)
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

// stubCommand simulates command execution
func stubCommand(cmd string) string {
  // TODO: Wire up actual shell execution
  // For now, return stubbed output
  return fmt.Sprintf("[Stub output for: %s]\n\nWire up PTY/shell execution here.", cmd)
}

// updateViewport rebuilds the viewport content
func (m *Model) updateViewport() {
  if !m.ready {
    return
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

    // Output
    if block.Output != "" {
      blockContent.WriteString(outputStyle.Render(block.Output))
    }

    // Wrap in styled block
    styledBlock := cmdBlockStyle.Render(blockContent.String())
    content.WriteString(styledBlock + "\n")
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
  if m.aiLoading {
    inputPrompt = fmt.Sprintf("%s Thinking... %s", m.spinner.View(), m.textInput.View())
  } else if m.aiMode {
    inputPrompt = fmt.Sprintf("✨ %s", m.textInput.View())
  } else {
    inputPrompt = m.textInput.View()
  }
  inputBar := inputContainerStyle.Width(m.width - 2).Render(inputPrompt)
  b.WriteString(inputBar + "\n")

  // Help bar
  help := helpStyle.Render("Ctrl+Space: AI mode • Esc: Exit AI mode • Ctrl+C: Quit")
  b.WriteString(help)

  return b.String()
}

func main() {
  p := tea.NewProgram(
    InitialModel(),
    tea.WithAltScreen(),
  )

  if _, err := p.Run(); err != nil {
    fmt.Printf("Error: %v", err)
  }
}
