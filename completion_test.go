package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestCompleteBuiltinCommands(t *testing.T) {
	// Test partial /command completion
	matches := getCompletions("/cl")
	found := false
	for _, m := range matches {
		if m == "/clear" {
			found = true
		}
	}
	if !found {
		t.Errorf("expected /clear in completions for '/cl', got %v", matches)
	}
}

func TestCompleteExactCommand(t *testing.T) {
	// Exact match should return nothing (no extra chars to add)
	matches := getCompletions("/clear")
	for _, m := range matches {
		if m == "/clear" {
			t.Error("should not return exact match as completion")
		}
	}
}

func TestCompleteFilePaths(t *testing.T) {
	dir := t.TempDir()
	// Create test files
	os.WriteFile(filepath.Join(dir, "readme.md"), nil, 0644)
	os.WriteFile(filepath.Join(dir, "main.go"), nil, 0644)
	os.MkdirAll(filepath.Join(dir, "src"), 0755)

	// Complete partial filename
	input := filepath.Join(dir, "rea")
	matches := getCompletions(input)
	if len(matches) == 0 {
		t.Fatalf("expected completions for '%s', got none", input)
	}
	found := false
	for _, m := range matches {
		if filepath.Base(m) == "readme.md" {
			found = true
		}
	}
	if !found {
		t.Errorf("expected readme.md in completions, got %v", matches)
	}
}

func TestCompleteDirectoryTrailingSlash(t *testing.T) {
	dir := t.TempDir()
	os.MkdirAll(filepath.Join(dir, "myproject"), 0755)

	input := filepath.Join(dir, "my")
	matches := getCompletions(input)
	if len(matches) == 0 {
		t.Fatalf("expected completions for '%s'", input)
	}
	found := false
	for _, m := range matches {
		if filepath.Base(m) == "myproject" && m[len(m)-1] == '/' {
			found = true
		}
	}
	if !found {
		t.Errorf("expected myproject/ with trailing slash, got %v", matches)
	}
}

func TestCompleteEmptyInput(t *testing.T) {
	matches := getCompletions("")
	if len(matches) != 0 {
		t.Errorf("expected no completions for empty input, got %v", matches)
	}
}

func TestCompleteWhitespaceInput(t *testing.T) {
	matches := getCompletions("   ")
	if len(matches) != 0 {
		t.Errorf("expected no completions for whitespace input, got %v", matches)
	}
}

func TestCompleteNonexistentPath(t *testing.T) {
	matches := getCompletions("/nonexistent/path/prefix")
	if len(matches) != 0 {
		t.Errorf("expected no completions for nonexistent path, got %v", matches)
	}
}

func TestCompleteBuiltinSearchCommand(t *testing.T) {
	matches := getCompletions("/se")
	found := false
	for _, m := range matches {
		if m == "/search" {
			found = true
		}
	}
	if !found {
		t.Errorf("expected /search in completions for '/se', got %v", matches)
	}
}
