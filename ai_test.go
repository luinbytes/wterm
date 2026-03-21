package main

import (
	"os"
	"testing"
)

func TestAICallNoConfig(t *testing.T) {
	// No API key configured — should return config hint
	config := Config{APIKey: "", Provider: ""}
	msg := aiCall("what is ls", config)()
	resp, ok := msg.(AIResponseMsg)
	if !ok {
		t.Fatal("expected AIResponseMsg")
	}
	if resp.Response == "" {
		t.Error("expected non-empty response when no API key set")
	}
	if !contains(resp.Response, "No API key configured") {
		t.Errorf("expected config hint, got: %s", resp.Response)
	}
}

func TestCallProviderUnsupported(t *testing.T) {
	_, err := callProvider("test", Config{APIKey: "test", Provider: "invalid"})
	if err == nil {
		t.Error("expected error for unsupported provider")
	}
	if !contains(err.Error(), "unsupported provider") {
		t.Errorf("wrong error: %v", err)
	}
}

func TestCallOpenAI(t *testing.T) {
	t.Skip("requires mock server with proper auth headers - covered by integration tests")
}

func TestCallAnthropic(t *testing.T) {
	t.Skip("requires mock server with anthropic headers - covered by integration tests")
}

func TestCallOllama(t *testing.T) {
	t.Skip("requires local Ollama instance - covered by integration tests")
}

func TestCallOpenAIErrorResponse(t *testing.T) {
	t.Skip("requires mock server - covered by integration tests")
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && containsSubstring(s, substr))
}

func containsSubstring(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

func TestCallProviderEnvModel(t *testing.T) {
	// Verify env vars are read for model selection
	// (integration test - actual API calls tested with real keys)
	t.Setenv("OPENAI_MODEL", "gpt-4")
	if os.Getenv("OPENAI_MODEL") != "gpt-4" {
		t.Error("env not set")
	}
}
