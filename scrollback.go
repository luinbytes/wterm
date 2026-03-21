package main

import "sync"

// Scrollback is a bounded ring buffer for command output history.
// It trims oldest entries when capacity is exceeded.
type Scrollback struct {
	mu       sync.RWMutex
	blocks   []CommandBlock
	maxSize  int
	scrollY  int // 0 = bottom (latest), increases upward
	totalWritten int // total blocks ever added (for offset math)
}

// NewScrollback creates a new scrollback buffer with the given max capacity.
func NewScrollback(maxSize int) *Scrollback {
	if maxSize <= 0 {
		maxSize = 1000
	}
	return &Scrollback{
		blocks:  make([]CommandBlock, 0, maxSize),
		maxSize: maxSize,
		scrollY: 0,
	}
}

// Append adds a command block to the scrollback, trimming oldest if at capacity.
func (s *Scrollback) Append(block CommandBlock) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.blocks = append(s.blocks, block)
	s.totalWritten++
	if len(s.blocks) > s.maxSize {
		// Trim oldest entries — keep only the last maxSize blocks
		trim := len(s.blocks) - s.maxSize
		s.blocks = s.blocks[trim:]
		// Adjust scrollY so it stays at the same relative position
		if s.scrollY > 0 {
			s.scrollY = max(0, s.scrollY-trim)
		}
	}
	// Auto-follow: new content scrolls to bottom
	s.scrollY = 0
}

// Blocks returns a copy of all stored blocks.
func (s *Scrollback) Blocks() []CommandBlock {
	s.mu.RLock()
	defer s.mu.RUnlock()
	out := make([]CommandBlock, len(s.blocks))
	copy(out, s.blocks)
	return out
}

// Len returns the number of stored blocks.
func (s *Scrollback) Len() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return len(s.blocks)
}

// Clear removes all blocks from the scrollback.
func (s *Scrollback) Clear() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.blocks = s.blocks[:0]
	s.scrollY = 0
}

// ScrollUp moves the scroll position up by n lines (toward older content).
// Returns the actual scroll position.
func (s *Scrollback) ScrollUp(n int) int {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.scrollY = min(s.scrollY+n, len(s.blocks)-1)
	return s.scrollY
}

// ScrollDown moves the scroll position down by n lines (toward newer content).
// Returns the actual scroll position.
func (s *Scrollback) ScrollDown(n int) int {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.scrollY = max(s.scrollY-n, 0)
	return s.scrollY
}

// ScrollToBottom resets scroll to the latest content.
func (s *Scrollback) ScrollToBottom() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.scrollY = 0
}

// ScrollY returns the current scroll offset.
func (s *Scrollback) ScrollY() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.scrollY
}

// IsAtBottom returns true if the viewport is at the latest content.
func (s *Scrollback) IsAtBottom() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.scrollY == 0
}

// VisibleBlocks returns the blocks that should be displayed given the current scroll position.
// offset is how many blocks from the end to skip (scrollY).
// If at bottom (scrollY=0), returns all blocks.
func (s *Scrollback) VisibleBlocks(viewHeight int) []CommandBlock {
	s.mu.RLock()
	defer s.mu.RUnlock()

	if s.scrollY == 0 {
		// At bottom — show all blocks (viewport handles its own scrolling)
		return s.blocks
	}

	// Scrolled up — return blocks starting from (len - scrollY)
	// The viewport shows these and the user can see older content
	start := len(s.blocks) - s.scrollY
	if start < 0 {
		start = 0
	}
	out := make([]CommandBlock, len(s.blocks)-start)
	copy(out, s.blocks[start:])
	return out
}

// SetMaxSize updates the maximum capacity, trimming if necessary.
func (s *Scrollback) SetMaxSize(maxSize int) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if maxSize <= 0 {
		maxSize = 1000
	}
	s.maxSize = maxSize
	if len(s.blocks) > s.maxSize {
		trim := len(s.blocks) - s.maxSize
		s.blocks = s.blocks[trim:]
		if s.scrollY > 0 {
			s.scrollY = max(0, s.scrollY-trim)
		}
	}
}

// MaxSize returns the current max capacity.
func (s *Scrollback) MaxSize() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.maxSize
}
