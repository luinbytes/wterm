package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

// AIRequest represents a chat completion request
type AIRequest struct {
	Model    string    `json:"model"`
	Messages []AIMsg   `json:"messages"`
	MaxTokens int      `json:"max_tokens,omitempty"`
}

// AIMsg represents a chat message
type AIMsg struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

// AIResponse represents a chat completion response (OpenAI-compatible)
type AIResponse struct {
	Choices []struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	} `json:"choices"`
	Error *struct {
		Message string `json:"message"`
	} `json:"error,omitempty"`
}

// aiCall makes a real API call to the configured AI provider.
// Falls back to stub behavior if no API key is configured.
func aiCall(prompt string, config Config) tea.Cmd {
	return func() tea.Msg {
		if config.APIKey == "" || config.Provider == "" {
			return AIResponseMsg{
				Response: fmt.Sprintf(
					"AI Response to: %q\n\n[No API key configured. Set apiKey and provider in ~/.wterm/config.yaml]\n\nSupported providers: openai, anthropic, ollama",
					prompt,
				),
			}
		}

		response, err := callProvider(prompt, config)
		if err != nil {
			return AIResponseMsg{
				Response: fmt.Sprintf("AI Error: %v", err),
			}
		}
		return AIResponseMsg{Response: response}
	}
}

// callProvider routes to the appropriate AI provider
func callProvider(prompt string, config Config) (string, error) {
	switch config.Provider {
	case "openai":
		return callOpenAI(prompt, config)
	case "anthropic":
		return callAnthropic(prompt, config)
	case "ollama":
		return callOllama(prompt, config)
	default:
		return "", fmt.Errorf("unsupported provider %q (use: openai, anthropic, ollama)", config.Provider)
	}
}

// callOpenAI makes a request to the OpenAI Chat Completions API
func callOpenAI(prompt string, config Config) (string, error) {
	model := "gpt-3.5-turbo"
	if env := os.Getenv("OPENAI_MODEL"); env != "" {
		model = env
	}

	reqBody := AIRequest{
		Model: model,
		Messages: []AIMsg{
			{Role: "system", Content: "You are a helpful terminal assistant. Explain shell commands concisely. Keep responses brief."},
			{Role: "user", Content: prompt},
		},
		MaxTokens: 500,
	}

	data, err := json.Marshal(reqBody)
	if err != nil {
		return "", fmt.Errorf("marshal request: %w", err)
	}

	req, err := http.NewRequest("POST", "https://api.openai.com/v1/chat/completions", bytes.NewReader(data))
	if err != nil {
		return "", fmt.Errorf("create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+config.APIKey)

	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return "", fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("read response: %w", err)
	}

	var aiResp AIResponse
	if err := json.Unmarshal(body, &aiResp); err != nil {
		return "", fmt.Errorf("parse response: %w", err)
	}

	if aiResp.Error != nil {
		return "", fmt.Errorf("API error: %s", aiResp.Error.Message)
	}

	if len(aiResp.Choices) == 0 {
		return "", fmt.Errorf("no response from API")
	}

	return strings.TrimSpace(aiResp.Choices[0].Message.Content), nil
}

// callAnthropic makes a request to the Anthropic Messages API
func callAnthropic(prompt string, config Config) (string, error) {
	model := "claude-3-haiku-20240307"
	if env := os.Getenv("ANTHROPIC_MODEL"); env != "" {
		model = env
	}

	type AnthropicReq struct {
		Model     string  `json:"model"`
		MaxTokens int     `json:"max_tokens"`
		System    string  `json:"system"`
		Messages  []AIMsg `json:"messages"`
	}

	type AnthropicResp struct {
		Content []struct {
			Text string `json:"text"`
		} `json:"content"`
		Error *struct {
			Message string `json:"message"`
		} `json:"error,omitempty"`
	}

	reqBody := AnthropicReq{
		Model:     model,
		MaxTokens: 500,
		System:    "You are a helpful terminal assistant. Explain shell commands concisely. Keep responses brief.",
		Messages:  []AIMsg{{Role: "user", Content: prompt}},
	}

	data, err := json.Marshal(reqBody)
	if err != nil {
		return "", fmt.Errorf("marshal request: %w", err)
	}

	req, err := http.NewRequest("POST", "https://api.anthropic.com/v1/messages", bytes.NewReader(data))
	if err != nil {
		return "", fmt.Errorf("create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("x-api-key", config.APIKey)
	req.Header.Set("anthropic-version", "2023-06-01")

	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return "", fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("read response: %w", err)
	}

	var aiResp AnthropicResp
	if err := json.Unmarshal(body, &aiResp); err != nil {
		return "", fmt.Errorf("parse response: %w", err)
	}

	if aiResp.Error != nil {
		return "", fmt.Errorf("API error: %s", aiResp.Error.Message)
	}

	if len(aiResp.Content) == 0 {
		return "", fmt.Errorf("no response from API")
	}

	return strings.TrimSpace(aiResp.Content[0].Text), nil
}

// callOllama makes a request to a local Ollama instance
func callOllama(prompt string, config Config) (string, error) {
	baseURL := "http://localhost:11434"
	if env := os.Getenv("OLLAMA_HOST"); env != "" {
		baseURL = env
	}

	model := "llama3"
	if env := os.Getenv("OLLAMA_MODEL"); env != "" {
		model = env
	}

	reqBody := AIRequest{
		Model: model,
		Messages: []AIMsg{
			{Role: "system", Content: "You are a helpful terminal assistant. Explain shell commands concisely. Keep responses brief."},
			{Role: "user", Content: prompt},
		},
	}

	data, err := json.Marshal(reqBody)
	if err != nil {
		return "", fmt.Errorf("marshal request: %w", err)
	}

	req, err := http.NewRequest("POST", baseURL+"/api/chat", bytes.NewReader(data))
	if err != nil {
		return "", fmt.Errorf("create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 60 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		return "", fmt.Errorf("failed to connect to Ollama at %s: %w", baseURL, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("read response: %w", err)
	}

	var aiResp AIResponse
	if err := json.Unmarshal(body, &aiResp); err != nil {
		return "", fmt.Errorf("parse response: %w", err)
	}

	if len(aiResp.Choices) == 0 {
		return "", fmt.Errorf("no response from Ollama (is the model running?)")
	}

	return strings.TrimSpace(aiResp.Choices[0].Message.Content), nil
}
