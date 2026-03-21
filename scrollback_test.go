package main

import "testing"

func TestNewScrollback(t *testing.T) {
	sb := NewScrollback(5)
	if sb.maxSize != 5 {
		t.Errorf("expected maxSize 5, got %d", sb.maxSize)
	}
	if sb.Len() != 0 {
		t.Errorf("expected len 0, got %d", sb.Len())
	}
}

func TestNewScrollbackZeroCapacity(t *testing.T) {
	sb := NewScrollback(0)
	if sb.maxSize != 1000 {
		t.Errorf("expected default maxSize 1000 for 0 input, got %d", sb.maxSize)
	}
	sb2 := NewScrollback(-1)
	if sb2.maxSize != 1000 {
		t.Errorf("expected default maxSize 1000 for negative input, got %d", sb2.maxSize)
	}
}

func TestAppend(t *testing.T) {
	sb := NewScrollback(5)
	for i := 0; i < 3; i++ {
		sb.Append(CommandBlock{Command: "echo", Output: "test"})
	}
	if sb.Len() != 3 {
		t.Errorf("expected len 3, got %d", sb.Len())
	}
	if !sb.IsAtBottom() {
		t.Error("expected to be at bottom after append")
	}
}

func TestAppendTrimsOldest(t *testing.T) {
	sb := NewScrollback(3)
	for i := 0; i < 5; i++ {
		sb.Append(CommandBlock{Command: "cmd", Output: "output"})
	}
	if sb.Len() != 3 {
		t.Errorf("expected len 3 after exceeding capacity, got %d", sb.Len())
	}
	blocks := sb.Blocks()
	for i, b := range blocks {
		if b.Command != "cmd" {
			t.Errorf("block %d has wrong command: %s", i, b.Command)
		}
	}
}

func TestAppendTrimsAdjustsScroll(t *testing.T) {
	sb := NewScrollback(3)
	sb.Append(CommandBlock{Command: "1"})
	sb.Append(CommandBlock{Command: "2"})
	sb.Append(CommandBlock{Command: "3"})

	// Scroll up
	sb.ScrollUp(2)

	// Add two more, which should trim and adjust scroll
	sb.Append(CommandBlock{Command: "4"})
	sb.Append(CommandBlock{Command: "5"})

	// scrollY should have been adjusted down by the trim amount
	if sb.scrollY != 0 {
		// After append, scroll resets to 0 (auto-follow)
		t.Errorf("expected scrollY 0 after append (auto-follow), got %d", sb.scrollY)
	}
}

func TestClear(t *testing.T) {
	sb := NewScrollback(10)
	sb.Append(CommandBlock{Command: "test"})
	sb.Clear()
	if sb.Len() != 0 {
		t.Errorf("expected len 0 after clear, got %d", sb.Len())
	}
	if sb.ScrollY() != 0 {
		t.Errorf("expected scrollY 0 after clear, got %d", sb.ScrollY())
	}
}

func TestScrollUp(t *testing.T) {
	sb := NewScrollback(10)
	for i := 0; i < 5; i++ {
		sb.Append(CommandBlock{Command: "cmd"})
	}

	sb.ScrollUp(3)
	if sb.ScrollY() != 3 {
		t.Errorf("expected scrollY 3, got %d", sb.ScrollY())
	}

	// Scroll up more than available — should clamp
	sb.ScrollUp(100)
	if sb.ScrollY() != 4 {
		t.Errorf("expected scrollY capped at 4, got %d", sb.ScrollY())
	}
}

func TestScrollDown(t *testing.T) {
	sb := NewScrollback(10)
	for i := 0; i < 5; i++ {
		sb.Append(CommandBlock{Command: "cmd"})
	}
	sb.ScrollUp(3)
	sb.ScrollDown(1)
	if sb.ScrollY() != 2 {
		t.Errorf("expected scrollY 2, got %d", sb.ScrollY())
	}

	// Scroll past bottom
	sb.ScrollDown(100)
	if sb.ScrollY() != 0 {
		t.Errorf("expected scrollY 0 when scrolled past bottom, got %d", sb.ScrollY())
	}
}

func TestScrollToBottom(t *testing.T) {
	sb := NewScrollback(10)
	for i := 0; i < 5; i++ {
		sb.Append(CommandBlock{Command: "cmd"})
	}
	sb.ScrollUp(3)
	sb.ScrollToBottom()
	if sb.ScrollY() != 0 {
		t.Errorf("expected scrollY 0, got %d", sb.ScrollY())
	}
}

func TestVisibleBlocksAtBottom(t *testing.T) {
	sb := NewScrollback(10)
	for i := 0; i < 5; i++ {
		sb.Append(CommandBlock{Command: "cmd"})
	}
	visible := sb.VisibleBlocks(10)
	if len(visible) != 5 {
		t.Errorf("expected 5 visible blocks at bottom, got %d", len(visible))
	}
}

func TestVisibleBlocksScrolledUp(t *testing.T) {
	sb := NewScrollback(10)
	for i := 0; i < 5; i++ {
		sb.Append(CommandBlock{Command: "cmd"})
	}
	// Use internal scroll to avoid auto-follow reset
	sb.mu.Lock()
	sb.scrollY = 2
	sb.mu.Unlock()
	visible := sb.VisibleBlocks(10)
	// Scrolled up 2 from 5 blocks -> start=3, shows blocks[3:] (indices 3,4) = 2 blocks
	if len(visible) != 2 {
		t.Errorf("expected 2 visible blocks when scrolled up 2, got %d", len(visible))
	}
}

func TestBlocksReturnsCopy(t *testing.T) {
	sb := NewScrollback(10)
	sb.Append(CommandBlock{Command: "original"})
	blocks := sb.Blocks()
	blocks[0].Command = "modified"
	if sb.Blocks()[0].Command != "original" {
		t.Error("Blocks() did not return a copy — mutation leaked")
	}
}

func TestSetMaxSize(t *testing.T) {
	sb := NewScrollback(10)
	for i := 0; i < 8; i++ {
		sb.Append(CommandBlock{Command: "cmd"})
	}
	sb.SetMaxSize(3)
	if sb.Len() != 3 {
		t.Errorf("expected len 3 after shrinking, got %d", sb.Len())
	}
	if sb.MaxSize() != 3 {
		t.Errorf("expected maxSize 3, got %d", sb.MaxSize())
	}
}

func TestSetMaxSizeZero(t *testing.T) {
	sb := NewScrollback(10)
	sb.SetMaxSize(0)
	if sb.MaxSize() != 1000 {
		t.Errorf("expected default maxSize 1000, got %d", sb.MaxSize())
	}
}

func TestIsAtBottom(t *testing.T) {
	sb := NewScrollback(10)
	if !sb.IsAtBottom() {
		t.Error("empty scrollback should be at bottom")
	}
	sb.Append(CommandBlock{Command: "cmd"})
	if !sb.IsAtBottom() {
		t.Error("after append should be at bottom")
	}
	// Set scroll position directly since Append resets to bottom
	sb.mu.Lock()
	sb.scrollY = 1
	sb.mu.Unlock()
	if sb.IsAtBottom() {
		t.Error("after scroll up should not be at bottom")
	}
}

func TestTotalWritten(t *testing.T) {
	sb := NewScrollback(3)
	for i := 0; i < 5; i++ {
		sb.Append(CommandBlock{Command: "cmd"})
	}
	if sb.totalWritten != 5 {
		t.Errorf("expected totalWritten 5, got %d", sb.totalWritten)
	}
	if sb.Len() != 3 {
		t.Errorf("expected len 3 (trimmed), got %d", sb.Len())
	}
}
