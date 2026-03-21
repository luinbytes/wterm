package main

import (
	"fmt"
	"strings"
	"testing"
)

// BenchmarkViewportBuild measures the time to build viewport content
// for different numbers of command blocks.
func BenchmarkViewportBuild(b *testing.B) {
	blocks := make([]CommandBlock, 0)
	// Generate realistic command blocks
	for i := 0; i < 500; i++ {
		var output string
		switch i % 5 {
		case 0:
			output = "file1.txt  file2.go  file3.md\nfile4.rs  file5.py"
		case 1:
			output = fmt.Sprintf("total %d\ndrwxr-xr-x  2 user user 4096 Jan 01 00:00 .\ndrwxr-xr-x 10 user user 4096 Jan 01 00:00 ..", 10+i%20)
		case 2:
			output = strings.Repeat("line of output text\n", 10+i%50)
		case 3:
			output = "Usage: cmd [options]\n  -h, --help     Show help\n  -v, --version  Show version"
		case 4:
			output = "OK\nDone in 0.123s"
		}
		blocks = append(blocks, CommandBlock{
			Command:  fmt.Sprintf("test command %d", i),
			Output:   output,
			IsAI:     i%10 == 0,
			ExitCode: i % 7,
		})
	}

	width := 80
	blockWidth := width - 6

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		// Simulate the viewport build loop from updateViewport
		content := strings.Builder{}
		content.Grow(len(blocks) * (blockWidth * 3))
		for _, block := range blocks {
			var blockContent strings.Builder
			var cmdLine string
			if block.IsAI {
				cmdLine = "AI " + block.Command
			} else {
				exitInd := ""
				if block.ExitCode != 0 {
					exitInd = fmt.Sprintf(" [%d]", block.ExitCode)
				}
				cmdLine = "> " + block.Command + exitInd
			}
			blockContent.WriteString(cmdLine + "\n")
			if block.Output != "" {
				blockContent.WriteString(block.Output)
			}
			content.WriteString(blockContent.String() + "\n")
		}
		_ = content.String()
	}
}

func BenchmarkViewportBuildWithoutPrealloc(b *testing.B) {
	blocks := make([]CommandBlock, 0)
	for i := 0; i < 500; i++ {
		output := strings.Repeat("line of output text\n", 10+i%50)
		blocks = append(blocks, CommandBlock{
			Command:  fmt.Sprintf("test command %d", i),
			Output:   output,
			ExitCode: i % 7,
		})
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		// Old approach: no pre-allocation
		var content strings.Builder
		for _, block := range blocks {
			var blockContent strings.Builder
			blockContent.WriteString("> " + block.Command + "\n")
			if block.Output != "" {
				blockContent.WriteString(block.Output)
			}
			content.WriteString(blockContent.String() + "\n")
		}
		_ = content.String()
	}
}
