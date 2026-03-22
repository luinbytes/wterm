package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func defaultTestConfig(path string) Config {
	return Config{
		MaxHistory: 100,
		History: HistoryConfig{
			PersistToFile: true,
			Path:          path,
			MaxFileSizeKB: 1024,
		},
	}
}

func TestSaveHistoryNoDuplication(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "history.txt")
	cfg := defaultTestConfig(path)

	// Simulate what happens during a session:
	// 1. Commands are appended one by one via appendToHistory
	if err := appendToHistory("cmd1", cfg); err != nil {
		t.Fatalf("appendToHistory cmd1 failed: %v", err)
	}
	if err := appendToHistory("cmd2", cfg); err != nil {
		t.Fatalf("appendToHistory cmd2 failed: %v", err)
	}
	if err := appendToHistory("cmd3", cfg); err != nil {
		t.Fatalf("appendToHistory cmd3 failed: %v", err)
	}

	// 2. On exit, saveHistory writes the full in-memory history
	err := saveHistory([]string{"cmd1", "cmd2", "cmd3"}, cfg)
	if err != nil {
		t.Fatalf("saveHistory failed: %v", err)
	}

	// 3. Verify no duplicates
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read history: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(data)), "\n")
	if len(lines) != 3 {
		t.Fatalf("expected 3 lines, got %d: %v", len(lines), lines)
	}

	expected := []string{"cmd1", "cmd2", "cmd3"}
	for i, line := range lines {
		if line != expected[i] {
			t.Errorf("line %d: expected %q, got %q", i, expected[i], line)
		}
	}
}

func TestSaveHistoryRewritePreservesState(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "history.txt")
	cfg := defaultTestConfig(path)

	// First session
	if err := appendToHistory("old-cmd", cfg); err != nil {
		t.Fatalf("appendToHistory failed: %v", err)
	}
	if err := saveHistory([]string{"old-cmd"}, cfg); err != nil {
		t.Fatalf("saveHistory failed: %v", err)
	}

	// Second session: user deletes old-cmd from memory (e.g., /clear-history)
	// saveHistory should reflect the current state, not accumulate
	err := saveHistory([]string{"new-cmd"}, cfg)
	if err != nil {
		t.Fatalf("saveHistory failed: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read history: %v", err)
	}

	content := strings.TrimSpace(string(data))
	if content != "new-cmd" {
		t.Errorf("expected only 'new-cmd', got: %q", content)
	}
}

func TestSaveHistoryTrimsToMaxHistory(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "history.txt")
	cfg := defaultTestConfig(path)
	cfg.MaxHistory = 3

	entries := []string{"a", "b", "c", "d", "e"}
	err := saveHistory(entries, cfg)
	if err != nil {
		t.Fatalf("saveHistory failed: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read history: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(data)), "\n")
	if len(lines) != 3 {
		t.Fatalf("expected 3 lines (trimmed to MaxHistory), got %d", len(lines))
	}
	if lines[0] != "c" || lines[2] != "e" {
		t.Errorf("expected last 3 entries [c,d,e], got %v", lines)
	}
}

func TestSaveHistoryEmptyList(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "history.txt")
	cfg := defaultTestConfig(path)

	// Write something first
	if err := appendToHistory("cmd", cfg); err != nil {
		t.Fatalf("appendToHistory failed: %v", err)
	}

	// Save empty history — should result in empty file
	err := saveHistory([]string{}, cfg)
	if err != nil {
		t.Fatalf("saveHistory failed: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("failed to read history: %v", err)
	}

	if strings.TrimSpace(string(data)) != "" {
		t.Errorf("expected empty file, got: %q", string(data))
	}
}

func TestSaveHistoryDisabled(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "history.txt")
	cfg := defaultTestConfig(path)
	cfg.History.PersistToFile = false

	err := saveHistory([]string{"cmd"}, cfg)
	if err != nil {
		t.Fatalf("saveHistory should be no-op when disabled: %v", err)
	}

	if _, err := os.Stat(path); !os.IsNotExist(err) {
		t.Error("history file should not exist when persistence is disabled")
	}
}
