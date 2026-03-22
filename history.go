package main

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// loadHistory loads command history from a file
func loadHistory(config Config) ([]string, error) {
	if !config.History.PersistToFile {
		return []string{}, nil
	}

	path := config.History.Path
	if path == "" {
		return []string{}, fmt.Errorf("history path is empty")
	}

	// Check if file exists
	if _, err := os.Stat(path); os.IsNotExist(err) {
		return []string{}, nil
	}

	// Open file
	file, err := os.Open(path)
	if err != nil {
		return []string{}, fmt.Errorf("failed to open history file: %w", err)
	}
	defer file.Close()

	// Check file size
	info, err := file.Stat()
	if err != nil {
		return []string{}, fmt.Errorf("failed to get file info: %w", err)
	}

	maxBytes := config.History.MaxFileSizeKB * 1024
	if info.Size() > int64(maxBytes) {
		// File is too large, truncate by reading only the last portion
		// We'll read line by line from the end
		return loadHistoryTruncated(file, config.MaxHistory)
	}

	// Read file line by line
	history := make([]string, 0, config.MaxHistory)
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line != "" {
			history = append(history, line)
		}
	}

	if err := scanner.Err(); err != nil {
		return []string{}, fmt.Errorf("failed to read history file: %w", err)
	}

	// Trim to max history size
	if len(history) > config.MaxHistory {
		history = history[len(history)-config.MaxHistory:]
	}

	return history, nil
}

// loadHistoryTruncated loads history from an oversized file by reading only the last N lines
func loadHistoryTruncated(file *os.File, maxLines int) ([]string, error) {
	// Seek to start of file
	if _, err := file.Seek(0, 0); err != nil {
		return []string{}, fmt.Errorf("failed to seek file: %w", err)
	}

	// Read all lines (file is oversized, but we need to parse it)
	var allLines []string
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line != "" {
			allLines = append(allLines, line)
		}
	}

	if err := scanner.Err(); err != nil {
		return []string{}, fmt.Errorf("failed to read history file: %w", err)
	}

	// Return only the last maxLines
	if len(allLines) > maxLines {
		return allLines[len(allLines)-maxLines:], nil
	}
	return allLines, nil
}

// saveHistory saves command history to a file
func saveHistory(history []string, config Config) error {
	if !config.History.PersistToFile {
		return nil
	}

	path := config.History.Path
	if path == "" {
		return fmt.Errorf("history path is empty")
	}

	// Ensure directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("failed to create history directory: %w", err)
	}

	// Trim history to max size before writing
	entries := history
	if len(entries) > config.MaxHistory {
		entries = entries[len(entries)-config.MaxHistory:]
	}

	// Truncate and rewrite the file with current in-memory history.
	// Since appendToHistory() already writes each command as it's entered,
	// we must rewrite (not append) to avoid duplicates on exit.
	file, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("failed to open history file: %w", err)
	}
	defer file.Close()

	for _, entry := range entries {
		if entry != "" {
			if _, err := file.WriteString(entry + "\n"); err != nil {
				return fmt.Errorf("failed to write to history file: %w", err)
			}
		}
	}

	return nil
}

// appendToHistory appends a single command to the history file
func appendToHistory(command string, config Config) error {
	if !config.History.PersistToFile || command == "" {
		return nil
	}

	path := config.History.Path
	if path == "" {
		return fmt.Errorf("history path is empty")
	}

	// Ensure directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("failed to create history directory: %w", err)
	}

	// Open file in append mode
	file, err := os.OpenFile(path, os.O_WRONLY|os.O_APPEND|os.O_CREATE, 0644)
	if err != nil {
		return fmt.Errorf("failed to open history file: %w", err)
	}
	defer file.Close()

	// Append command
	if _, err := file.WriteString(command + "\n"); err != nil {
		return fmt.Errorf("failed to write to history file: %w", err)
	}

	return nil
}
