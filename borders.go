package main

import (
	"runtime"

	"github.com/charmbracelet/lipgloss"
)

// safeBorder returns RoundedBorder on Unix and NormalBorder on Windows.
// RoundedBorder uses Unicode box-drawing chars (╭╮╰╯─│) that can render
// as broken pipe characters on Windows even with VT processing enabled.
func safeBorder() lipgloss.Border {
	if runtime.GOOS == "windows" {
		return lipgloss.HiddenBorder()
	}
	return lipgloss.RoundedBorder()
}
