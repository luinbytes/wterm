//go:build !windows

package main

// setupConsole is a no-op on non-Windows platforms.
// Unix-like systems generally support UTF-8 by default.
func setupConsole() {
	// No setup needed on Unix-like systems
}
