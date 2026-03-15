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

	// Check if file exceeds size limit and truncate if needed
	if _, err := os.Stat(path); err == nil {
		info, _ := os.Stat(path)
		maxBytes := config.History.MaxFileSizeKB * 1024
		if info.Size() > int64(maxBytes) {
			// Truncate file by keeping only recent history
			if err := truncateHistoryFile(path, config.MaxHistory); err != nil {
				return fmt.Errorf("failed to truncate history file: %w", err)
			}
		}
	}

	// Open file in append mode
	file, err := os.OpenFile(path, os.O_WRONLY|os.O_APPEND|os.O_CREATE, 0644)
	if err != nil {
		return fmt.Errorf("failed to open history file: %w", err)
	}
	defer file.Close()

	// Append new entries
	// Only write the last batch of entries that aren't already in the file
	// For simplicity, we'll append all current history
	// In a production system, you'd want to track which entries are new
	for _, entry := range history {
		if entry != "" {
			if _, err := file.WriteString(entry + "\n"); err != nil {
				return fmt.Errorf("failed to write to history file: %w", err)
			}
		}
	}

	return nil
}

// truncateHistoryFile truncates the history file to keep only the last N lines
func truncateHistoryFile(path string, maxLines int) error {
	// Read all lines
	file, err := os.Open(path)
	if err != nil {
		return err
	}
	defer file.Close()

	var lines []string
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line != "" {
			lines = append(lines, line)
		}
	}

	if err := scanner.Err(); err != nil {
		return err
	}

	// Keep only last maxLines
	if len(lines) > maxLines {
		lines = lines[len(lines)-maxLines:]
	}

	// Write back
	file, err = os.Create(path)
	if err != nil {
		return err
	}
	defer file.Close()

	for _, line := range lines {
		if _, err := file.WriteString(line + "\n"); err != nil {
			return err
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
