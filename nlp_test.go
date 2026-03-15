package main

import (
	"runtime"
	"strings"
	"testing"
)

// TestNLPParserNavigation tests navigation-related patterns
func TestNLPParserNavigation(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name          string
		input         string
		expectMatch   bool
		expectCmd     string
		skipIfWindows bool
	}{
		{
			name:        "go to directory",
			input:       "go to /home/user",
			expectMatch: true,
			expectCmd:   `cd "/home/user"`,
		},
		{
			name:        "go to with spaces",
			input:       "go to my folder",
			expectMatch: true,
			expectCmd:   `cd "my folder"`,
		},
		{
			name:        "go back",
			input:       "go back",
			expectMatch: true,
			expectCmd:   "cd ..",
		},
		{
			name:        "show current directory",
			input:       "show current directory",
			expectMatch: true,
		},
		{
			name:        "where am i",
			input:       "where am i",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.skipIfWindows && runtime.GOOS == "windows" {
				t.Skip("Skipping on Windows")
			}

			cmd, matched, desc := parser.Parse(tt.input)

			if !matched {
				t.Errorf("Expected pattern to match, but it didn't")
				return
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}

			if tt.expectCmd != "" && cmd != tt.expectCmd {
				t.Errorf("Expected command %q, got %q", tt.expectCmd, cmd)
			}
		})
	}
}

// TestNLPParserFiles tests file operation patterns
func TestNLPParserFiles(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
		expectCmd   string
	}{
		{
			name:        "list files",
			input:       "list files",
			expectMatch: true,
		},
		{
			name:        "list files sorted by name",
			input:       "list files sorted by name",
			expectMatch: true,
		},
		{
			name:        "list files sorted by size",
			input:       "list files sorted by size",
			expectMatch: true,
		},
		{
			name:        "list files sorted by date",
			input:       "list files sorted by date",
			expectMatch: true,
		},
		{
			name:        "create folder",
			input:       "create folder test",
			expectMatch: true,
			expectCmd:   `mkdir "test"`,
		},
		{
			name:        "create directory",
			input:       "create directory mydir",
			expectMatch: true,
			expectCmd:   `mkdir "mydir"`,
		},
		{
			name:        "delete file",
			input:       "delete file test.txt",
			expectMatch: true,
		},
		{
			name:        "delete the file",
			input:       "delete the file test.txt",
			expectMatch: true,
		},
		{
			name:        "copy file to",
			input:       "copy test.txt to backup.txt",
			expectMatch: true,
		},
		{
			name:        "copy file into",
			input:       "copy test.txt into backup.txt",
			expectMatch: true,
		},
		{
			name:        "move file to",
			input:       "move test.txt to backup/",
			expectMatch: true,
		},
		{
			name:        "move file into",
			input:       "move test.txt into backup/",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cmd, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
				return
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}

			if tt.expectCmd != "" && cmd != tt.expectCmd {
				t.Errorf("Expected command %q, got %q", tt.expectCmd, cmd)
			}
		})
	}
}

// TestNLPParserSystem tests system-related patterns
func TestNLPParserSystem(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
	}{
		{
			name:        "open file",
			input:       "open test.txt",
			expectMatch: true,
		},
		{
			name:        "clear screen",
			input:       "clear",
			expectMatch: true,
		},
		{
			name:        "clean screen",
			input:       "clean",
			expectMatch: true,
		},
		{
			name:        "show disk space",
			input:       "show disk space",
			expectMatch: true,
		},
		{
			name:        "show ip address",
			input:       "show ip address",
			expectMatch: true,
		},
		{
			name:        "show my ip",
			input:       "show my ip",
			expectMatch: true,
		},
		{
			name:        "show date",
			input:       "show date",
			expectMatch: true,
		},
		{
			name:        "show time",
			input:       "show time",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserSearch tests search-related patterns
func TestNLPParserSearch(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
	}{
		{
			name:        "find files named",
			input:       "find files named *.go",
			expectMatch: true,
		},
		{
			name:        "find files containing",
			input:       "find files containing test",
			expectMatch: true,
		},
		{
			name:        "find files with",
			input:       "find files with .txt",
			expectMatch: true,
		},
		{
			name:        "find text in files",
			input:       "find text hello in files",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserProcess tests process-related patterns
func TestNLPParserProcess(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
	}{
		{
			name:        "show processes",
			input:       "show processes",
			expectMatch: true,
		},
		{
			name:        "show running processes",
			input:       "show running processes",
			expectMatch: true,
		},
		{
			name:        "show process",
			input:       "show process",
			expectMatch: true,
		},
		{
			name:        "kill process",
			input:       "kill process 1234",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserEnvironment tests environment-related patterns
func TestNLPParserEnvironment(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
	}{
		{
			name:        "set variable",
			input:       "set variable PATH to /usr/bin",
			expectMatch: true,
		},
		{
			name:        "set env var",
			input:       "set API_KEY to secret",
			expectMatch: true,
		},
		{
			name:        "show variable",
			input:       "show variable PATH",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserNetwork tests network-related patterns
func TestNLPParserNetwork(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
	}{
		{
			name:        "ping host",
			input:       "ping google.com",
			expectMatch: true,
		},
		{
			name:        "trace route",
			input:       "trace route to google.com",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserTextFile tests text file operation patterns
func TestNLPParserTextFile(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
	}{
		{
			name:        "show file",
			input:       "show file test.txt",
			expectMatch: true,
		},
		{
			name:        "read file",
			input:       "read file test.txt",
			expectMatch: true,
		},
		{
			name:        "display file",
			input:       "display file test.txt",
			expectMatch: true,
		},
		{
			name:        "cat file",
			input:       "cat file test.txt",
			expectMatch: true,
		},
		{
			name:        "edit file",
			input:       "edit file test.txt",
			expectMatch: true,
		},
		{
			name:        "open file for edit",
			input:       "open file test.txt",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserCaseInsensitive tests that patterns are case-insensitive
func TestNLPParserCaseInsensitive(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name  string
		input string
	}{
		{"lowercase", "list files"},
		{"uppercase", "LIST FILES"},
		{"mixed case", "List Files"},
		{"title case", "List Files"},
		{"random case", "lIsT fIlEs"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if !matched {
				t.Errorf("Expected pattern to match for case-insensitive input: %q", tt.input)
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserUnmatchedInput tests that unmatched input passes through
func TestNLPParserUnmatchedInput(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name  string
		input string
	}{
		{"random command", "ls -la"},
		{"pipe command", "cat file.txt | grep test"},
		{"shell command", "echo 'hello world'"},
		{"unknown phrase", "make it so"},
		{"gibberish", "asdfjkl"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cmd, matched, desc := parser.Parse(tt.input)

			// Unmatched input should return the original input as the command
			if matched {
				t.Errorf("Expected pattern to NOT match, but it matched with desc: %s", desc)
			}

			if cmd != tt.input {
				t.Errorf("Expected unmatched input to pass through as-is: got %q, want %q", cmd, tt.input)
			}
		})
	}
}

// TestNLPParserEmptyInput tests empty string handling
func TestNLPParserEmptyInput(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name  string
		input string
	}{
		{"empty string", ""},
		{"whitespace only", "   "},
		{"tabs only", "\t\t"},
		{"newlines only", "\n\n"},
		{"mixed whitespace", "  \t\n  "},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cmd, matched, desc := parser.Parse(tt.input)

			if matched {
				t.Errorf("Expected empty/whitespace input to NOT match")
			}

			if cmd != "" {
				t.Errorf("Expected empty command for empty/whitespace input: got %q", cmd)
			}

			if desc != "" {
				t.Errorf("Expected empty description for empty/whitespace input: got %q", desc)
			}
		})
	}
}

// TestNLPParserAliases tests Unix command aliases
func TestNLPParserAliases(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name        string
		input       string
		expectMatch bool
	}{
		{
			name:        "ls alias",
			input:       "ls",
			expectMatch: true,
		},
		{
			name:        "pwd alias",
			input:       "pwd",
			expectMatch: true,
		},
		{
			name:        "mkdir alias",
			input:       "mkdir testdir",
			expectMatch: true,
		},
		{
			name:        "rm alias",
			input:       "rm test.txt",
			expectMatch: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, matched, desc := parser.Parse(tt.input)

			if tt.expectMatch && !matched {
				t.Errorf("Expected pattern to match, but it didn't")
			}

			if matched && desc == "" {
				t.Errorf("Expected description to be set for matched pattern")
			}
		})
	}
}

// TestNLPParserPlatformSpecific tests platform-specific command generation
func TestNLPParserPlatformSpecific(t *testing.T) {
	parser := NewNLPParser()

	t.Run("list files platform", func(t *testing.T) {
		_, matched, desc := parser.Parse("list files")
		if !matched {
			t.Errorf("Expected pattern to match")
		}

		if desc == "" {
			t.Errorf("Expected description to be set")
		}
	})

	t.Run("pwd platform", func(t *testing.T) {
		cmd, matched, desc := parser.Parse("where am i")
		if !matched {
			t.Errorf("Expected pattern to match")
		}

		if desc == "" {
			t.Errorf("Expected description to be set")
		}

		// Verify platform-appropriate command
		if runtime.GOOS == "windows" && cmd != "cd" {
			t.Errorf("Expected 'cd' command on Windows, got: %s", cmd)
		}
		if runtime.GOOS != "windows" && cmd != "pwd" {
			t.Errorf("Expected 'pwd' command on Unix, got: %s", cmd)
		}
	})
}

// TestNLPParserPatternCount verifies the number of registered patterns
func TestNLPParserPatternCount(t *testing.T) {
	parser := NewNLPParser()

	// The parser should have a reasonable number of patterns registered
	// This test helps ensure we don't accidentally break pattern registration
	if len(parser.patterns) < 20 {
		t.Errorf("Expected at least 20 patterns, got %d", len(parser.patterns))
	}

	// Also verify all patterns have descriptions
	for i, pattern := range parser.patterns {
		if pattern.Description == "" {
			t.Errorf("Pattern at index %d is missing description", i)
		}
	}
}

// TestNLPParserTrimmedInput tests that whitespace is trimmed properly
func TestNLPParserTrimmedInput(t *testing.T) {
	parser := NewNLPParser()

	tests := []struct {
		name  string
		input string
		want  string
	}{
		{
			name:  "trailing spaces",
			input: "list files   ",
			want:  "list files",
		},
		{
			name:  "leading spaces",
			input: "  list files",
			want:  "list files",
		},
		{
			name:  "both sides",
			input: "  list files  ",
			want:  "list files",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cmd, matched, _ := parser.Parse(tt.input)

			// For matched patterns, the command should be the generated one
			// For unmatched patterns, it should be the trimmed input
			if !matched {
				if cmd != strings.TrimSpace(tt.input) {
					t.Errorf("Expected trimmed input: got %q, want %q", cmd, tt.want)
				}
			}
		})
	}
}
