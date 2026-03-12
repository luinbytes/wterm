//go:build windows

package main

import (
	"syscall"
)

var (
	kernel32           = syscall.NewLazyDLL("kernel32.dll")
	setConsoleOutputCP = kernel32.NewProc("SetConsoleOutputCP")
	setConsoleCP       = kernel32.NewProc("SetConsoleCP")
)

// setupConsole configures the Windows console for proper Unicode output.
// This fixes broken border rendering by setting the code page to UTF-8 (65001).
func setupConsole() {
	// Set console output code page to UTF-8 (65001)
	setConsoleOutputCP.Call(uintptr(65001))
	// Set console input code page to UTF-8 (65001)
	setConsoleCP.Call(uintptr(65001))
}
