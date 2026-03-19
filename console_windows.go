//go:build windows

package main

import (
	"syscall"
	"unsafe"
)

var (
	kernel32           = syscall.NewLazyDLL("kernel32.dll")
	setConsoleOutputCP = kernel32.NewProc("SetConsoleOutputCP")
	setConsoleCP       = kernel32.NewProc("SetConsoleCP")
	getConsoleMode     = kernel32.NewProc("GetConsoleMode")
	setConsoleMode     = kernel32.NewProc("SetConsoleMode")
	getStdHandle       = kernel32.NewProc("GetStdHandle")
)

const (
	STD_OUTPUT_HANDLE                  = ^uintptr(10) // -11
	ENABLE_VIRTUAL_TERMINAL_PROCESSING = 0x0004
)

// setupConsole configures the Windows console for proper Unicode and ANSI output.
// This fixes broken border rendering by setting the code page to UTF-8 (65001)
// and enabling Virtual Terminal Processing for ANSI/VT100 escape sequences.
func setupConsole() {
	// Set console output code page to UTF-8 (65001)
	setConsoleOutputCP.Call(uintptr(65001))
	// Set console input code page to UTF-8 (65001)
	setConsoleCP.Call(uintptr(65001))

	// Enable VT processing so box-drawing/Unicode borders render correctly
	// This allows lipgloss's RoundedBorder() to draw ╭ ╮ ╰ ╯ ─ │ properly
	handle, _, _ := getStdHandle.Call(STD_OUTPUT_HANDLE)
	var mode uint32
	getConsoleMode.Call(handle, uintptr(unsafe.Pointer(&mode)))
	setConsoleMode.Call(handle, uintptr(mode|ENABLE_VIRTUAL_TERMINAL_PROCESSING))
}
