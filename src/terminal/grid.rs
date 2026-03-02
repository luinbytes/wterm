//! Terminal screen buffer (grid) for storing and manipulating terminal cell data.
//!
//! This module provides a 2D grid representation of terminal content,
//! including character data, colors, text attributes, and scrollback history.
#![allow(dead_code)]

use std::fmt;

use super::parser::{Color, TerminalOutput, TextAttributes};

/// A single cell in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The character displayed in this cell.
    pub char: char,
    /// Foreground color.
    pub fg_color: Color,
    /// Background color.
    pub bg_color: Color,
    /// Text attributes (bold, underline, etc.).
    pub attributes: TextAttributes,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg_color: Color::Default,
            bg_color: Color::Default,
            attributes: TextAttributes::default(),
        }
    }
}

impl Cell {
    /// Create a new cell with the given character.
    pub fn new(char: char) -> Self {
        Self {
            char,
            ..Self::default()
        }
    }

    /// Create a new cell with character and colors.
    pub fn with_colors(char: char, fg_color: Color, bg_color: Color) -> Self {
        Self {
            char,
            fg_color,
            bg_color,
            attributes: TextAttributes::default(),
        }
    }

    /// Create a fully specified cell.
    pub fn with_attributes(
        char: char,
        fg_color: Color,
        bg_color: Color,
        attributes: TextAttributes,
    ) -> Self {
        Self {
            char,
            fg_color,
            bg_color,
            attributes,
        }
    }

    /// Check if this cell is empty (space with default colors and attributes).
    pub fn is_empty(&self) -> bool {
        self.char == ' '
            && self.fg_color == Color::Default
            && self.bg_color == Color::Default
            && self.attributes == TextAttributes::default()
    }

    /// Reset the cell to default state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Cursor position in the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    /// 0-indexed row.
    pub row: usize,
    /// 0-indexed column.
    pub col: usize,
}

impl Cursor {
    /// Create a new cursor at the given position.
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    /// Create a cursor at the origin (0, 0).
    pub fn origin() -> Self {
        Self::default()
    }
}

/// A row in the scrollback buffer.
type ScrollbackRow = Vec<Cell>;

/// Represents a dirty region in the grid that needs to be redrawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRegion {
    /// Top row of dirty region (inclusive)
    pub top: usize,
    /// Bottom row of dirty region (inclusive)
    pub bottom: usize,
    /// Left column of dirty region (inclusive)
    pub left: usize,
    /// Right column of dirty region (inclusive)
    pub right: usize,
    /// Whether the entire screen is dirty
    pub full_screen: bool,
}

impl DirtyRegion {
    /// Create a new dirty region for a single cell
    pub fn single_cell(row: usize, col: usize) -> Self {
        Self {
            top: row,
            bottom: row,
            left: col,
            right: col,
            full_screen: false,
        }
    }

    /// Create a dirty region for an entire row
    pub fn full_row(row: usize, cols: usize) -> Self {
        Self {
            top: row,
            bottom: row,
            left: 0,
            right: cols.saturating_sub(1),
            full_screen: false,
        }
    }

    /// Mark the entire screen as dirty
    pub fn full_screen() -> Self {
        Self {
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
            full_screen: true,
        }
    }

    /// Check if the region is empty (no dirty cells)
    pub fn is_empty(&self) -> bool {
        !self.full_screen && self.top > self.bottom
    }

    /// Reset the dirty region to empty
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Expand this dirty region to include another region
    pub fn merge(&mut self, other: &DirtyRegion) {
        if other.full_screen {
            *self = DirtyRegion::full_screen();
            return;
        }
        if self.full_screen {
            return;
        }
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            *self = *other;
            return;
        }
        self.top = self.top.min(other.top);
        self.bottom = self.bottom.max(other.bottom);
        self.left = self.left.min(other.left);
        self.right = self.right.max(other.right);
    }

    /// Expand this dirty region to include a cell
    pub fn include_cell(&mut self, row: usize, col: usize) {
        if self.full_screen {
            return;
        }
        if self.is_empty() {
            *self = DirtyRegion::single_cell(row, col);
            return;
        }
        self.top = self.top.min(row);
        self.bottom = self.bottom.max(row);
        self.left = self.left.min(col);
        self.right = self.right.max(col);
    }
}

impl Default for DirtyRegion {
    fn default() -> Self {
        Self {
            top: 1,
            bottom: 0,
            left: 0,
            right: 0,
            full_screen: false,
        }
    }
}

/// Terminal screen buffer with scrollback history.
///
/// This struct manages a 2D grid of cells representing the visible terminal
/// content, plus an optional scrollback buffer for history.
#[derive(Debug, Clone)]
pub struct TerminalGrid {
    /// The visible grid of cells (rows × cols).
    grid: Vec<Vec<Cell>>,
    /// Number of columns (width).
    cols: usize,
    /// Number of rows (height).
    rows: usize,
    /// Current cursor position.
    cursor: Cursor,
    /// Scrollback buffer (lines that have scrolled off the top).
    scrollback: Vec<ScrollbackRow>,
    /// Maximum number of scrollback lines to keep.
    max_scrollback: usize,
    /// Current text attributes for new characters.
    attributes: TextAttributes,
    /// Current foreground color.
    fg_color: Color,
    /// Current background color.
    bg_color: Color,
    /// Dirty region tracking for efficient rendering
    dirty_region: DirtyRegion,
    /// Whether batch updates are currently enabled
    batch_updates: bool,
    /// Pending cell updates during batch mode
    batch_buffer: Vec<(usize, usize, Cell)>,
    /// Scroll offset for navigating scrollback (0 = bottom/normal view)
    scroll_offset: usize,
}

impl TerminalGrid {
    /// Create a new terminal grid with default dimensions (80×24).
    pub fn new() -> Self {
        Self::with_size(80, 24)
    }

    /// Create a new terminal grid with specified dimensions.
    ///
    /// # Arguments
    /// * `cols` - Number of columns (width).
    /// * `rows` - Number of rows (height).
    pub fn with_size(cols: usize, rows: usize) -> Self {
        Self {
            grid: vec![vec![Cell::default(); cols]; rows],
            cols,
            rows,
            cursor: Cursor::default(),
            scrollback: Vec::new(),
            max_scrollback: 10000,
            attributes: TextAttributes::default(),
            fg_color: Color::Default,
            bg_color: Color::Default,
            dirty_region: DirtyRegion::full_screen(),
            batch_updates: false,
            batch_buffer: Vec::new(),
            scroll_offset: 0,
        }
    }

    /// Create a terminal grid with custom scrollback size.
    ///
    /// # Arguments
    /// * `cols` - Number of columns (width).
    /// * `rows` - Number of rows (height).
    /// * `max_scrollback` - Maximum scrollback lines to retain.
    pub fn with_scrollback(cols: usize, rows: usize, max_scrollback: usize) -> Self {
        Self {
            grid: vec![vec![Cell::default(); cols]; rows],
            cols,
            rows,
            cursor: Cursor::default(),
            scrollback: Vec::new(),
            max_scrollback,
            attributes: TextAttributes::default(),
            fg_color: Color::Default,
            bg_color: Color::Default,
            dirty_region: DirtyRegion::full_screen(),
            batch_updates: false,
            batch_buffer: Vec::new(),
            scroll_offset: 0,
        }
    }

    /// Get the number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get the number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get the current cursor position.
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Get the number of scrollback lines.
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Get a row from the scrollback buffer by index.
    pub fn get_scrollback_row(&self, index: usize) -> Option<&ScrollbackRow> {
        self.scrollback.get(index)
    }

    /// Get the current scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scroll up in history (towards older content).
    ///
    /// Returns true if the scroll position changed.
    pub fn scroll_up_history(&mut self, lines: usize) -> bool {
        let max_offset = self.scrollback.len();
        let new_offset = (self.scroll_offset + lines).min(max_offset);
        if new_offset != self.scroll_offset {
            self.scroll_offset = new_offset;
            self.dirty_region = DirtyRegion::full_screen();
            true
        } else {
            false
        }
    }

    /// Scroll down in history (towards newer content).
    ///
    /// Returns true if the scroll position changed.
    pub fn scroll_down_history(&mut self, lines: usize) -> bool {
        let new_offset = self.scroll_offset.saturating_sub(lines);
        if new_offset != self.scroll_offset {
            self.scroll_offset = new_offset;
            self.dirty_region = DirtyRegion::full_screen();
            true
        } else {
            false
        }
    }

    /// Scroll to the bottom (reset scroll offset).
    pub fn scroll_to_bottom(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset = 0;
            self.dirty_region = DirtyRegion::full_screen();
        }
    }

    /// Scroll to the top of scrollback.
    pub fn scroll_to_top(&mut self) {
        let max_offset = self.scrollback.len();
        if self.scroll_offset != max_offset {
            self.scroll_offset = max_offset;
            self.dirty_region = DirtyRegion::full_screen();
        }
    }

    /// Check if we're at the bottom (not scrolled).
    pub fn is_at_bottom(&self) -> bool {
        self.scroll_offset == 0
    }

    /// Get a cell from the visible area, considering scroll offset.
    ///
    /// When scrolled, this returns cells from the scrollback buffer for
    /// the upper portion of the screen.
    pub fn get_cell_with_scroll(&self, row: usize, col: usize) -> Option<&Cell> {
        if col >= self.cols {
            return None;
        }

        if self.scroll_offset == 0 {
            // Normal view - just get from the grid
            self.grid.get(row).and_then(|r| r.get(col))
        } else {
            // Scrolled view - mix scrollback and grid
            let scrollback_len = self.scrollback.len();
            let scrolled_rows = self.scroll_offset.min(scrollback_len);

            if row < scrolled_rows {
                // This row is from scrollback
                let scrollback_idx = scrollback_len - scrolled_rows + row;
                self.scrollback.get(scrollback_idx)
                    .and_then(|row_cells| row_cells.get(col))
            } else {
                // This row is from the grid
                let grid_row = row - scrolled_rows;
                self.grid.get(grid_row).and_then(|r| r.get(col))
            }
        }
    }

    /// Set the current text attributes.
    pub fn set_attributes(&mut self, attributes: TextAttributes) {
        self.attributes = attributes;
    }

    /// Get the current text attributes.
    pub fn attributes(&self) -> TextAttributes {
        self.attributes
    }

    /// Set the current foreground color.
    pub fn set_foreground(&mut self, color: Color) {
        self.fg_color = color;
    }

    /// Set the current background color.
    pub fn set_background(&mut self, color: Color) {
        self.bg_color = color;
    }

    /// Move the cursor to a specific position.
    ///
    /// The position is clamped to valid grid coordinates.
    pub fn move_cursor(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.rows.saturating_sub(1));
        self.cursor.col = col.min(self.cols.saturating_sub(1));
    }

    /// Move the cursor relative to its current position.
    pub fn move_cursor_relative(&mut self, row_delta: isize, col_delta: isize) {
        if row_delta < 0 {
            self.cursor.row = self.cursor.row.saturating_sub(row_delta.unsigned_abs());
        } else {
            self.cursor.row =
                (self.cursor.row + row_delta as usize).min(self.rows.saturating_sub(1));
        }

        if col_delta < 0 {
            self.cursor.col = self.cursor.col.saturating_sub(col_delta.unsigned_abs());
        } else {
            self.cursor.col =
                (self.cursor.col + col_delta as usize).min(self.cols.saturating_sub(1));
        }
    }

    /// Put a character at the current cursor position and advance the cursor.
    ///
    /// If at the end of a line (cursor past last column), this wraps to the next line first.
    /// If at the bottom of the screen, this scrolls up.
    pub fn put_char(&mut self, c: char) {
        // Handle pending wrap (cursor past last column)
        if self.cursor.col >= self.cols {
            self.cursor.col = 0;
            self.cursor.row += 1;

            // Scroll if needed
            if self.cursor.row >= self.rows {
                self.scroll_up(1);
                self.cursor.row = self.rows - 1;
            }
        }

        // Write the character at current cursor position
        if self.cursor.row < self.rows && self.cursor.col < self.cols {
            let cell = Cell {
                char: c,
                fg_color: self.fg_color,
                bg_color: self.bg_color,
                attributes: self.attributes,
            };
            self.apply_cell_update(self.cursor.row, self.cursor.col, cell);
        }

        // Advance cursor
        self.cursor.col += 1;
    }

    /// Put a character without advancing the cursor.
    pub fn put_char_at(&mut self, row: usize, col: usize, c: char) {
        if row < self.rows && col < self.cols {
            let cell = Cell {
                char: c,
                fg_color: self.fg_color,
                bg_color: self.bg_color,
                attributes: self.attributes,
            };
            self.apply_cell_update(row, col, cell);
        }
    }

    /// Get a cell at the given position.
    ///
    /// Returns None if the position is out of bounds.
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.grid.get(row)?.get(col)
    }

    /// Get a mutable reference to a cell at the given position.
    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.grid.get_mut(row)?.get_mut(col)
    }

    /// Get a reference to a row.
    pub fn get_row(&self, row: usize) -> Option<&[Cell]> {
        self.grid.get(row).map(|r| r.as_slice())
    }

    /// Resize the terminal grid.
    ///
    /// Content is preserved where possible. New cells are initialized to default.
    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        // Resize each row
        for row in &mut self.grid {
            row.resize(new_cols, Cell::default());
        }

        // Add or remove rows
        self.grid.resize(new_rows, vec![Cell::default(); new_cols]);

        self.cols = new_cols;
        self.rows = new_rows;

        // Clamp cursor to new dimensions
        self.cursor.row = self.cursor.row.min(self.rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(self.cols.saturating_sub(1));
    }

    /// Clear the entire screen, filling with default cells.
    pub fn clear_screen(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                cell.reset();
            }
        }
        // Also clear scrollback when clearing screen
        self.scrollback.clear();
        // Mark entire screen as dirty
        self.mark_full_dirty();
    }

    /// Clear the screen but preserve scrollback.
    pub fn clear_screen_keep_scrollback(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                cell.reset();
            }
        }
        // Mark entire screen as dirty
        self.mark_full_dirty();
    }

    /// Clear from cursor to end of screen.
    pub fn clear_to_end_of_screen(&mut self) {
        // Clear from cursor to end of current line
        self.clear_to_end_of_line();

        // Clear all lines below
        for row_idx in (self.cursor.row + 1)..self.rows {
            for cell in &mut self.grid[row_idx] {
                cell.reset();
            }
        }
        // Mark region as dirty
        self.mark_region_dirty(self.cursor.row, self.rows - 1, 0, self.cols - 1);
    }

    /// Clear from start of screen to cursor.
    pub fn clear_to_start_of_screen(&mut self) {
        // Clear all lines above
        for row_idx in 0..self.cursor.row {
            for cell in &mut self.grid[row_idx] {
                cell.reset();
            }
        }

        // Clear from start of current line to cursor
        self.clear_to_start_of_line();
        // Mark region as dirty
        self.mark_region_dirty(0, self.cursor.row, 0, self.cursor.col);
    }

    /// Clear the current line.
    pub fn clear_line(&mut self) {
        if self.cursor.row < self.rows {
            for cell in &mut self.grid[self.cursor.row] {
                cell.reset();
            }
            // Mark row as dirty
            self.mark_region_dirty(self.cursor.row, self.cursor.row, 0, self.cols - 1);
        }
    }

    /// Clear from cursor to end of the current line.
    pub fn clear_to_end_of_line(&mut self) {
        if self.cursor.row < self.rows {
            for col_idx in self.cursor.col..self.cols {
                self.grid[self.cursor.row][col_idx].reset();
            }
            // Mark region as dirty
            self.mark_region_dirty(
                self.cursor.row,
                self.cursor.row,
                self.cursor.col,
                self.cols - 1,
            );
        }
    }

    /// Clear from start of the current line to cursor.
    pub fn clear_to_start_of_line(&mut self) {
        if self.cursor.row < self.rows {
            for col_idx in 0..=self.cursor.col.min(self.cols - 1) {
                self.grid[self.cursor.row][col_idx].reset();
            }
            // Mark region as dirty
            self.mark_region_dirty(self.cursor.row, self.cursor.row, 0, self.cursor.col);
        }
    }

    /// Scroll the screen up by n lines.
    ///
    /// Lines that scroll off the top are moved to the scrollback buffer.
    /// Note: This does NOT adjust the cursor position - callers must do that if needed.
    pub fn scroll_up(&mut self, n: usize) {
        if n == 0 || self.rows == 0 {
            return;
        }

        let scroll_amount = n.min(self.rows);

        // Move scrolled lines to scrollback
        for i in 0..scroll_amount {
            if self.scrollback.len() >= self.max_scrollback {
                self.scrollback.remove(0);
            }
            // Clone the row before it gets replaced
            let row = self.grid[i].clone();
            self.scrollback.push(row);
        }

        // Shift rows up
        self.grid.drain(0..scroll_amount);

        // Add new empty rows at the bottom
        for _ in 0..scroll_amount {
            self.grid.push(vec![Cell::default(); self.cols]);
        }

        // Reset scroll offset when new content arrives
        self.scroll_offset = 0;

        // Mark entire screen as dirty after scroll
        self.mark_full_dirty();
    }

    /// Scroll the screen down by n lines.
    ///
    /// This is the opposite of scroll_up - new lines appear at the top.
    pub fn scroll_down(&mut self, n: usize) {
        if n == 0 || self.rows == 0 {
            return;
        }

        let scroll_amount = n.min(self.rows);

        // Remove rows from the bottom
        let rows_to_remove = self.rows.saturating_sub(scroll_amount);
        self.grid.drain(rows_to_remove..);

        // Add new empty rows at the top
        for _ in 0..scroll_amount {
            self.grid.insert(0, vec![Cell::default(); self.cols]);
        }

        // Adjust cursor position
        self.cursor.row = (self.cursor.row + scroll_amount).min(self.rows.saturating_sub(1));

        // Mark entire screen as dirty after scroll
        self.mark_full_dirty();
    }

    /// Scroll a region of the screen up by n lines.
    ///
    /// Only lines within [top, bottom] are affected. Lines above `top` and
    /// below `bottom` remain unchanged. The bottom `n` lines of the region
    /// are cleared.
    ///
    /// # Arguments
    /// * `n` - Number of lines to scroll
    /// * `top` - Top boundary of scroll region (0-indexed, inclusive)
    /// * `bottom` - Bottom boundary of scroll region (0-indexed, inclusive)
    pub fn scroll_up_in_region(&mut self, n: usize, top: usize, bottom: usize) {
        if n == 0 || self.rows == 0 || top >= bottom || bottom >= self.rows {
            return;
        }

        let region_height = bottom - top + 1;
        let scroll_amount = n.min(region_height);

        // Save rows outside the region
        let above_region: Vec<Vec<Cell>> = if top > 0 {
            self.grid[0..top].to_vec()
        } else {
            Vec::new()
        };

        let mut in_region: Vec<Vec<Cell>> = self.grid[top..=bottom].to_vec();

        // Shift region content up
        in_region.drain(0..scroll_amount);

        // Add blank lines at bottom of region
        for _ in 0..scroll_amount {
            in_region.push(vec![Cell::default(); self.cols]);
        }

        // Reconstruct grid
        let mut new_grid = above_region;
        new_grid.append(&mut in_region);

        // Add rows below region
        if bottom + 1 < self.rows {
            new_grid.extend_from_slice(&self.grid[bottom + 1..]);
        }

        self.grid = new_grid;

        // Mark the scrolled region as dirty
        self.mark_region_dirty(top, bottom, 0, self.cols - 1);
    }

    /// Scroll a region of the screen down by n lines.
    ///
    /// Only lines within [top, bottom] are affected. Lines above `top` and
    /// below `bottom` remain unchanged. The top `n` lines of the region
    /// are cleared.
    ///
    /// # Arguments
    /// * `n` - Number of lines to scroll
    /// * `top` - Top boundary of scroll region (0-indexed, inclusive)
    /// * `bottom` - Bottom boundary of scroll region (0-indexed, inclusive)
    pub fn scroll_down_in_region(&mut self, n: usize, top: usize, bottom: usize) {
        if n == 0 || self.rows == 0 || top >= bottom || bottom >= self.rows {
            return;
        }

        let region_height = bottom - top + 1;
        let scroll_amount = n.min(region_height);

        // Save rows outside the region
        let above_region: Vec<Vec<Cell>> = if top > 0 {
            self.grid[0..top].to_vec()
        } else {
            Vec::new()
        };

        let mut in_region: Vec<Vec<Cell>> = self.grid[top..=bottom].to_vec();

        // Remove lines from bottom of region
        let remove_from = in_region.len().saturating_sub(scroll_amount);
        in_region.drain(remove_from..);

        // Insert blank lines at top of region
        for _ in 0..scroll_amount {
            in_region.insert(0, vec![Cell::default(); self.cols]);
        }

        // Reconstruct grid
        let mut new_grid = above_region;
        new_grid.append(&mut in_region);

        // Add rows below region
        if bottom + 1 < self.rows {
            new_grid.extend_from_slice(&self.grid[bottom + 1..]);
        }

        self.grid = new_grid;

        // Mark the scrolled region as dirty
        self.mark_region_dirty(top, bottom, 0, self.cols - 1);
    }

    /// Perform a line feed within a scroll region.
    ///
    /// If cursor is at the bottom of the region, scroll the region up.
    /// Otherwise, just move the cursor down.
    pub fn linefeed_in_region(&mut self, top: usize, bottom: usize) {
        if self.cursor.row == bottom {
            // At bottom of region, scroll up
            self.scroll_up_in_region(1, top, bottom);
        } else if self.cursor.row < self.rows - 1 {
            self.cursor.row += 1;
        }
    }

    /// Erase in display with mode.
    ///
    /// Mode 0: Erase from cursor to end of screen
    /// Mode 1: Erase from start of screen to cursor  
    /// Mode 2: Erase entire screen
    /// Mode 3: Erase entire screen and scrollback
    pub fn erase_in_display(&mut self, mode: u16) {
        match mode {
            0 => self.clear_to_end_of_screen(),
            1 => self.clear_to_start_of_screen(),
            2 | 3 => {
                self.clear_screen();
                if mode == 3 {
                    self.scrollback.clear();
                }
            }
            _ => {}
        }
    }

    /// Erase in line with mode.
    ///
    /// Mode 0: Erase from cursor to end of line
    /// Mode 1: Erase from start of line to cursor
    /// Mode 2: Erase entire line
    pub fn erase_in_line(&mut self, mode: u16) {
        match mode {
            0 => self.clear_to_end_of_line(),
            1 => self.clear_to_start_of_line(),
            2 => self.clear_line(),
            _ => {}
        }
    }

    /// Insert n blank lines at the cursor position.
    ///
    /// Lines below the cursor are shifted down, and lines that fall off
    /// the bottom are lost.
    pub fn insert_lines(&mut self, n: usize) {
        if self.cursor.row >= self.rows {
            return;
        }

        let insert_count = n.min(self.rows - self.cursor.row);

        // Remove lines from the bottom to make room
        let lines_to_remove = (self.cursor.row + insert_count).saturating_sub(self.rows);
        if lines_to_remove > 0 {
            self.grid.drain((self.rows - lines_to_remove)..);
        }

        // Insert blank lines at cursor position
        for _ in 0..insert_count {
            self.grid
                .insert(self.cursor.row, vec![Cell::default(); self.cols]);
        }

        // Ensure we still have the right number of rows
        self.grid.truncate(self.rows);
    }

    /// Delete n lines at the cursor position.
    ///
    /// Lines below the deleted lines are shifted up, and blank lines
    /// appear at the bottom.
    pub fn delete_lines(&mut self, n: usize) {
        if self.cursor.row >= self.rows {
            return;
        }

        let delete_count = n.min(self.rows - self.cursor.row);

        // Remove lines at cursor position
        self.grid
            .drain(self.cursor.row..(self.cursor.row + delete_count));

        // Add blank lines at the bottom
        for _ in 0..delete_count {
            self.grid.push(vec![Cell::default(); self.cols]);
        }
    }

    /// Perform a line feed (move cursor down, possibly scrolling).
    pub fn linefeed(&mut self) {
        if self.cursor.row < self.rows - 1 {
            self.cursor.row += 1;
        } else {
            self.scroll_up(1);
        }
    }

    /// Perform a carriage return (move cursor to column 0).
    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
    }

    /// Backspace (move cursor left by one, but not past column 0).
    pub fn backspace(&mut self) {
        self.cursor.col = self.cursor.col.saturating_sub(1);
    }

    /// Tab (move cursor to next tab stop, every 8 columns).
    pub fn tab(&mut self) {
        self.cursor.col = ((self.cursor.col / 8) + 1) * 8;
        if self.cursor.col >= self.cols {
            self.cursor.col = self.cols - 1;
        }
    }

    /// Back tab (move cursor to previous tab stop).
    pub fn back_tab(&mut self) {
        self.cursor.col = ((self.cursor.col.saturating_sub(1)) / 8) * 8;
    }

    /// Get the entire visible grid as a slice of rows.
    pub fn as_rows(&self) -> &[Vec<Cell>] {
        &self.grid
    }

    /// Get the scrollback buffer.
    pub fn scrollback(&self) -> &[ScrollbackRow] {
        &self.scrollback
    }

    /// Clear the scrollback buffer.
    pub fn clear_scrollback(&mut self) {
        self.scrollback.clear();
    }

    /// Get the content of a row as a string.
    pub fn row_to_string(&self, row: usize) -> String {
        if row >= self.rows {
            return String::new();
        }

        self.grid[row].iter().map(|cell| cell.char).collect()
    }

    /// Save the current cursor position and attributes.
    pub fn save_cursor(&mut self) -> (Cursor, TextAttributes, Color, Color) {
        (self.cursor, self.attributes, self.fg_color, self.bg_color)
    }

    /// Restore a previously saved cursor position and attributes.
    pub fn restore_cursor(&mut self, saved: (Cursor, TextAttributes, Color, Color)) {
        self.cursor = saved.0;
        self.attributes = saved.1;
        self.fg_color = saved.2;
        self.bg_color = saved.3;
    }

    /// Reset the grid to initial state.
    pub fn reset(&mut self) {
        self.clear_screen();
        self.cursor = Cursor::default();
        self.attributes = TextAttributes::default();
        self.fg_color = Color::Default;
        self.bg_color = Color::Default;
    }

    // ===== Dirty Region Tracking =====

    /// Get the current dirty region.
    pub fn dirty_region(&self) -> &DirtyRegion {
        &self.dirty_region
    }

    /// Check if there are any dirty regions to render.
    pub fn has_dirty_regions(&self) -> bool {
        !self.dirty_region.is_empty()
    }

    /// Clear the dirty region (mark everything as clean).
    pub fn clear_dirty(&mut self) {
        self.dirty_region.clear();
    }

    /// Mark the entire screen as dirty.
    pub fn mark_full_dirty(&mut self) {
        self.dirty_region = DirtyRegion::full_screen();
    }

    /// Mark a specific cell as dirty.
    fn mark_cell_dirty(&mut self, row: usize, col: usize) {
        if self.batch_updates {
            // During batch mode, defer dirty tracking
            return;
        }
        self.dirty_region.include_cell(row, col);
    }

    /// Mark a region as dirty.
    fn mark_region_dirty(&mut self, top: usize, bottom: usize, left: usize, right: usize) {
        if self.batch_updates {
            // During batch mode, defer dirty tracking
            return;
        }
        self.dirty_region.merge(&DirtyRegion {
            top,
            bottom,
            left,
            right,
            full_screen: false,
        });
    }

    // ===== Batch Updates =====

    /// Start batch update mode.
    ///
    /// While in batch mode, dirty region tracking is deferred and cell updates
    /// are buffered. Call `flush_batch()` to apply all updates and calculate the
    /// final dirty region.
    pub fn begin_batch(&mut self) {
        self.batch_updates = true;
        self.batch_buffer.clear();
    }

    /// End batch update mode and flush all pending updates.
    ///
    /// This applies all buffered cell updates and calculates the dirty region
    /// covering all modified cells.
    pub fn flush_batch(&mut self) {
        self.batch_updates = false;

        // Apply all buffered updates
        for (row, col, cell) in self.batch_buffer.drain(..) {
            if row < self.rows && col < self.cols {
                self.grid[row][col] = cell;
                self.dirty_region.include_cell(row, col);
            }
        }
    }

    /// Cancel batch update mode without applying updates.
    pub fn cancel_batch(&mut self) {
        self.batch_updates = false;
        self.batch_buffer.clear();
    }

    /// Check if currently in batch mode.
    pub fn is_in_batch_mode(&self) -> bool {
        self.batch_updates
    }

    /// Internal method to apply a cell update (respects batch mode)
    fn apply_cell_update(&mut self, row: usize, col: usize, cell: Cell) {
        if self.batch_updates {
            // Buffer the update
            self.batch_buffer.push((row, col, cell));
        } else {
            // Apply immediately
            if row < self.rows && col < self.cols {
                self.grid[row][col] = cell;
                self.mark_cell_dirty(row, col);
            }
        }
    }
}

impl Default for TerminalGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TerminalGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content: String = self
            .grid
            .iter()
            .map(|row| row.iter().map(|cell| cell.char).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        write!(f, "{}", content)
    }
}

/// Implement TerminalOutput trait to allow the parser to write directly to the grid.
impl TerminalOutput for TerminalGrid {
    fn put_char(&mut self, c: char) {
        // Use the grid's put_char which handles wrapping and scrolling
        self.put_char(c);
    }

    fn backspace(&mut self) {
        self.backspace();
    }

    fn tab(&mut self) {
        self.tab();
    }

    fn linefeed_in_region(&mut self, top: usize, bottom: usize) {
        self.linefeed_in_region(top, bottom);
    }

    fn carriage_return(&mut self) {
        self.carriage_return();
    }

    fn move_cursor(&mut self, row: usize, col: usize) {
        self.move_cursor(row, col);
    }

    fn clear_screen(&mut self) {
        self.clear_screen();
    }

    fn cursor_position(&self) -> (usize, usize) {
        (self.cursor.row, self.cursor.col)
    }

    fn scroll_up_in_region(&mut self, n: usize, top: usize, bottom: usize) {
        self.scroll_up_in_region(n, top, bottom);
    }

    fn scroll_down_in_region(&mut self, n: usize, top: usize, bottom: usize) {
        self.scroll_down_in_region(n, top, bottom);
    }

    fn erase_in_display(&mut self, mode: u16) {
        self.erase_in_display(mode);
    }

    fn erase_in_line(&mut self, mode: u16) {
        self.erase_in_line(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = TerminalGrid::new();
        assert_eq!(grid.cols(), 80);
        assert_eq!(grid.rows(), 24);
        assert_eq!(grid.cursor(), Cursor::origin());
    }

    #[test]
    fn test_grid_with_size() {
        let grid = TerminalGrid::with_size(120, 40);
        assert_eq!(grid.cols(), 120);
        assert_eq!(grid.rows(), 40);
    }

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert_eq!(cell.char, ' ');
        assert_eq!(cell.fg_color, Color::Default);
        assert_eq!(cell.bg_color, Color::Default);
        assert!(cell.is_empty());
    }

    #[test]
    fn test_put_char() {
        let mut grid = TerminalGrid::with_size(10, 5);

        grid.put_char('H');
        assert_eq!(grid.cursor().col, 1);
        assert_eq!(grid.get_cell(0, 0).unwrap().char, 'H');

        grid.put_char('i');
        assert_eq!(grid.cursor().col, 2);
        assert_eq!(grid.get_cell(0, 1).unwrap().char, 'i');
    }

    #[test]
    fn test_put_char_with_color() {
        let mut grid = TerminalGrid::new();
        grid.set_foreground(Color::Indexed(1));
        grid.set_background(Color::Rgb(0, 0, 0));

        grid.put_char('X');
        let cell = grid.get_cell(0, 0).unwrap();
        assert_eq!(cell.char, 'X');
        assert_eq!(cell.fg_color, Color::Indexed(1));
        assert_eq!(cell.bg_color, Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_put_char_with_attributes() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        attrs.underline = true;
        grid.set_attributes(attrs);

        grid.put_char('B');
        let cell = grid.get_cell(0, 0).unwrap();
        assert!(cell.attributes.bold);
        assert!(cell.attributes.underline);
    }

    #[test]
    fn test_move_cursor() {
        let mut grid = TerminalGrid::new();

        grid.move_cursor(5, 10);
        assert_eq!(grid.cursor().row, 5);
        assert_eq!(grid.cursor().col, 10);

        // Test clamping
        grid.move_cursor(100, 100);
        assert_eq!(grid.cursor().row, 23); // rows - 1
        assert_eq!(grid.cursor().col, 79); // cols - 1
    }

    #[test]
    fn test_move_cursor_relative() {
        let mut grid = TerminalGrid::new();

        grid.move_cursor(10, 10);
        grid.move_cursor_relative(2, 5);
        assert_eq!(grid.cursor().row, 12);
        assert_eq!(grid.cursor().col, 15);

        grid.move_cursor_relative(-5, -3);
        assert_eq!(grid.cursor().row, 7);
        assert_eq!(grid.cursor().col, 12);

        // Test clamping
        grid.move_cursor_relative(-100, -100);
        assert_eq!(grid.cursor().row, 0);
        assert_eq!(grid.cursor().col, 0);
    }

    #[test]
    fn test_line_wrapping() {
        let mut grid = TerminalGrid::with_size(5, 3);

        // Write 5 characters to fill first line
        for c in "ABCDE".chars() {
            grid.put_char(c);
        }
        assert_eq!(grid.cursor().row, 0);
        assert_eq!(grid.cursor().col, 5); // At end of line

        // One more character should wrap
        grid.put_char('F');
        assert_eq!(grid.cursor().row, 1);
        assert_eq!(grid.cursor().col, 1);

        // Check content
        assert_eq!(grid.row_to_string(0), "ABCDE");
        assert_eq!(grid.get_cell(1, 0).unwrap().char, 'F');
    }

    #[test]
    fn test_scroll_up() {
        let mut grid = TerminalGrid::with_size(5, 3);

        // Fill grid with identifiable content
        grid.move_cursor(0, 0);
        for c in "AAAAA".chars() {
            grid.put_char(c);
        }
        grid.move_cursor(1, 0);
        for c in "BBBBB".chars() {
            grid.put_char(c);
        }
        grid.move_cursor(2, 0);
        for c in "CCCCC".chars() {
            grid.put_char(c);
        }

        // Scroll up 1 line
        grid.scroll_up(1);

        // Check that B is now at row 0, C at row 1, and row 2 is empty
        assert_eq!(grid.row_to_string(0), "BBBBB");
        assert_eq!(grid.row_to_string(1), "CCCCC");
        assert_eq!(grid.row_to_string(2), "     ");

        // Check scrollback
        assert_eq!(grid.scrollback_len(), 1);
        assert_eq!(
            grid.scrollback()[0]
                .iter()
                .map(|c| c.char)
                .collect::<String>(),
            "AAAAA"
        );
    }

    #[test]
    fn test_clear_screen() {
        let mut grid = TerminalGrid::with_size(5, 3);

        // Write some content
        grid.put_char('X');
        grid.put_char('Y');
        grid.put_char('Z');

        grid.clear_screen();

        // All cells should be empty
        for row in grid.as_rows() {
            for cell in row {
                assert!(cell.is_empty());
            }
        }
    }

    #[test]
    fn test_clear_line() {
        let mut grid = TerminalGrid::with_size(5, 3);

        // Fill first two rows
        grid.move_cursor(0, 0);
        for c in "AAAAA".chars() {
            grid.put_char(c);
        }
        grid.move_cursor(1, 0);
        for c in "BBBBB".chars() {
            grid.put_char(c);
        }

        // Clear first row
        grid.move_cursor(0, 0);
        grid.clear_line();

        assert_eq!(grid.row_to_string(0), "     ");
        assert_eq!(grid.row_to_string(1), "BBBBB");
    }

    #[test]
    fn test_clear_to_end_of_line() {
        let mut grid = TerminalGrid::with_size(5, 3);

        grid.move_cursor(0, 0);
        for c in "ABCDE".chars() {
            grid.put_char(c);
        }

        // Clear from column 2 to end
        grid.move_cursor(0, 2);
        grid.clear_to_end_of_line();

        assert_eq!(grid.row_to_string(0), "AB   ");
    }

    #[test]
    fn test_resize() {
        let mut grid = TerminalGrid::with_size(10, 5);

        // Write some content
        grid.move_cursor(0, 0);
        for c in "ABCDEFGHIJ".chars() {
            grid.put_char(c);
        }

        // Resize larger
        grid.resize(15, 8);
        assert_eq!(grid.cols(), 15);
        assert_eq!(grid.rows(), 8);

        // Original content should be preserved
        assert_eq!(grid.row_to_string(0), "ABCDEFGHIJ     ");

        // Resize smaller
        grid.resize(5, 3);
        assert_eq!(grid.cols(), 5);
        assert_eq!(grid.rows(), 3);
        assert_eq!(grid.row_to_string(0), "ABCDE");
    }

    #[test]
    fn test_scrollback() {
        let mut grid = TerminalGrid::with_scrollback(5, 2, 10);

        // Fill first row
        grid.move_cursor(0, 0);
        for c in "AAAAA".chars() {
            grid.put_char(c);
        }

        // Fill second row
        grid.move_cursor(1, 0);
        for c in "BBBBB".chars() {
            grid.put_char(c);
        }

        // One more line should scroll
        grid.move_cursor(1, 0);
        for c in "CCCCC".chars() {
            grid.put_char(c);
        }

        // Scroll up
        grid.scroll_up(1);

        assert_eq!(grid.scrollback_len(), 1);
        assert_eq!(
            grid.scrollback()[0]
                .iter()
                .map(|c| c.char)
                .collect::<String>(),
            "AAAAA"
        );
    }

    #[test]
    fn test_carriage_return() {
        let mut grid = TerminalGrid::new();
        grid.move_cursor(5, 10);

        grid.carriage_return();
        assert_eq!(grid.cursor().col, 0);
        assert_eq!(grid.cursor().row, 5); // Row unchanged
    }

    #[test]
    fn test_linefeed() {
        let mut grid = TerminalGrid::with_size(5, 3);

        grid.move_cursor(0, 2);
        grid.linefeed();
        assert_eq!(grid.cursor().row, 1);
        assert_eq!(grid.cursor().col, 2); // Col unchanged

        // At bottom, should scroll
        grid.move_cursor(2, 0);
        for c in "LAST!".chars() {
            grid.put_char(c);
        }
        grid.linefeed();
        assert_eq!(grid.cursor().row, 2); // Still at bottom after scroll
        assert_eq!(grid.scrollback_len(), 1);
    }

    #[test]
    fn test_backspace() {
        let mut grid = TerminalGrid::new();
        grid.move_cursor(5, 10);

        grid.backspace();
        assert_eq!(grid.cursor().col, 9);

        // At column 0, should stay at 0
        grid.move_cursor(5, 0);
        grid.backspace();
        assert_eq!(grid.cursor().col, 0);
    }

    #[test]
    fn test_tab() {
        let mut grid = TerminalGrid::with_size(20, 5);

        grid.move_cursor(0, 0);
        grid.tab();
        assert_eq!(grid.cursor().col, 8);

        grid.tab();
        assert_eq!(grid.cursor().col, 16);

        grid.tab();
        assert_eq!(grid.cursor().col, 19); // Clamped to max
    }

    #[test]
    fn test_insert_delete_lines() {
        let mut grid = TerminalGrid::with_size(5, 5);

        // Fill with identifiable content
        for (row, ch) in ['A', 'B', 'C', 'D', 'E'].iter().enumerate() {
            grid.move_cursor(row, 0);
            for _ in 0..5 {
                grid.put_char(*ch);
            }
        }

        // Insert 2 blank lines at row 2
        grid.move_cursor(2, 0);
        grid.insert_lines(2);

        assert_eq!(grid.row_to_string(0), "AAAAA");
        assert_eq!(grid.row_to_string(1), "BBBBB");
        assert_eq!(grid.row_to_string(2), "     "); // Inserted blank
        assert_eq!(grid.row_to_string(3), "     "); // Inserted blank
        assert_eq!(grid.row_to_string(4), "CCCCC"); // Shifted up
                                                    // D and E are lost

        // Delete 1 line at row 0
        grid.move_cursor(0, 0);
        grid.delete_lines(1);

        assert_eq!(grid.row_to_string(0), "BBBBB"); // Shifted up
    }

    #[test]
    fn test_save_restore_cursor() {
        let mut grid = TerminalGrid::new();

        grid.move_cursor(10, 20);
        grid.set_foreground(Color::Indexed(5));
        grid.set_background(Color::Rgb(100, 100, 100));
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        grid.set_attributes(attrs);

        let saved = grid.save_cursor();

        // Change state
        grid.move_cursor(0, 0);
        grid.set_foreground(Color::Default);
        grid.set_background(Color::Default);
        grid.set_attributes(TextAttributes::default());

        // Restore
        grid.restore_cursor(saved);

        assert_eq!(grid.cursor().row, 10);
        assert_eq!(grid.cursor().col, 20);
        assert_eq!(grid.fg_color, Color::Indexed(5));
        assert_eq!(grid.bg_color, Color::Rgb(100, 100, 100));
        assert!(grid.attributes().bold);
    }

    #[test]
    fn test_to_string() {
        let mut grid = TerminalGrid::with_size(5, 2);

        grid.move_cursor(0, 0);
        for c in "Hello".chars() {
            grid.put_char(c);
        }
        grid.move_cursor(1, 0);
        for c in "World".chars() {
            grid.put_char(c);
        }

        assert_eq!(grid.to_string(), "Hello\nWorld");
    }

    #[test]
    fn test_scrollback_limit() {
        let mut grid = TerminalGrid::with_scrollback(5, 2, 3);

        // Scroll more than max_scrollback
        for ch in ['A', 'B', 'C', 'D', 'E'] {
            grid.scroll_up(1);
            grid.move_cursor(0, 0);
            for _ in 0..5 {
                grid.put_char(ch);
            }
        }

        // Should only keep last 3
        assert_eq!(grid.scrollback_len(), 3);
    }

    #[test]
    fn test_get_cell_out_of_bounds() {
        let grid = TerminalGrid::with_size(10, 5);

        assert!(grid.get_cell(0, 0).is_some());
        assert!(grid.get_cell(4, 9).is_some());
        assert!(grid.get_cell(5, 0).is_none()); // Row out of bounds
        assert!(grid.get_cell(0, 10).is_none()); // Col out of bounds
    }

    #[test]
    fn test_reset() {
        let mut grid = TerminalGrid::with_size(5, 3);

        grid.move_cursor(2, 4);
        grid.set_foreground(Color::Indexed(1));
        grid.set_background(Color::Rgb(50, 50, 50));
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        grid.set_attributes(attrs);

        grid.put_char('X');

        grid.reset();

        assert_eq!(grid.cursor(), Cursor::default());
        assert_eq!(grid.fg_color, Color::Default);
        assert_eq!(grid.bg_color, Color::Default);
        assert_eq!(grid.attributes(), TextAttributes::default());
        assert!(grid.scrollback().is_empty());

        // All cells should be empty
        for row in grid.as_rows() {
            for cell in row {
                assert!(cell.is_empty());
            }
        }
    }

    // ===== Scroll Region Tests =====

    #[test]
    fn test_scroll_up_in_region() {
        let mut grid = TerminalGrid::with_size(5, 10);

        // Fill grid with identifiable content (0-9)
        for row in 0..10 {
            grid.move_cursor(row, 0);
            let ch = (b'0' + row as u8) as char;
            for _ in 0..5 {
                grid.put_char(ch);
            }
        }

        // Scroll region 3-6 (0-indexed) up by 1
        grid.scroll_up_in_region(1, 3, 6);

        // Check rows outside region are unchanged
        assert_eq!(grid.row_to_string(0), "00000");
        assert_eq!(grid.row_to_string(1), "11111");
        assert_eq!(grid.row_to_string(2), "22222");

        // Row 3 should now have content from row 4
        assert_eq!(grid.row_to_string(3), "44444");
        assert_eq!(grid.row_to_string(4), "55555");
        assert_eq!(grid.row_to_string(5), "66666");

        // Row 6 (bottom of region) should be cleared
        assert_eq!(grid.row_to_string(6), "     ");

        // Rows below region unchanged
        assert_eq!(grid.row_to_string(7), "77777");
        assert_eq!(grid.row_to_string(8), "88888");
        assert_eq!(grid.row_to_string(9), "99999");
    }

    #[test]
    fn test_scroll_down_in_region() {
        let mut grid = TerminalGrid::with_size(5, 10);

        // Fill grid with identifiable content (0-9)
        for row in 0..10 {
            grid.move_cursor(row, 0);
            let ch = (b'0' + row as u8) as char;
            for _ in 0..5 {
                grid.put_char(ch);
            }
        }

        // Scroll region 3-6 (0-indexed) down by 1
        grid.scroll_down_in_region(1, 3, 6);

        // Check rows outside region are unchanged
        assert_eq!(grid.row_to_string(0), "00000");
        assert_eq!(grid.row_to_string(1), "11111");
        assert_eq!(grid.row_to_string(2), "22222");

        // Row 3 (top of region) should be cleared
        assert_eq!(grid.row_to_string(3), "     ");

        // Rows 4-6 should have content from rows 3-5
        assert_eq!(grid.row_to_string(4), "33333");
        assert_eq!(grid.row_to_string(5), "44444");
        assert_eq!(grid.row_to_string(6), "55555");

        // Rows below region unchanged
        assert_eq!(grid.row_to_string(7), "77777");
        assert_eq!(grid.row_to_string(8), "88888");
        assert_eq!(grid.row_to_string(9), "99999");
    }

    #[test]
    fn test_scroll_region_boundaries() {
        let mut grid = TerminalGrid::with_size(5, 5);

        // Fill grid
        for row in 0..5 {
            grid.move_cursor(row, 0);
            let ch = (b'A' + row as u8) as char;
            for _ in 0..5 {
                grid.put_char(ch);
            }
        }

        // Invalid: top >= bottom - should be no-op
        grid.scroll_up_in_region(1, 3, 2);
        assert_eq!(grid.row_to_string(0), "AAAAA");
        assert_eq!(grid.row_to_string(3), "DDDDD");

        // Invalid: n == 0 - should be no-op
        grid.scroll_up_in_region(0, 0, 4);
        assert_eq!(grid.row_to_string(0), "AAAAA");
    }

    #[test]
    fn test_linefeed_in_region() {
        let mut grid = TerminalGrid::with_size(5, 10);

        // Fill region 2-5
        for row in 2..=5 {
            grid.move_cursor(row, 0);
            let ch = (b'0' + row as u8) as char;
            for _ in 0..5 {
                grid.put_char(ch);
            }
        }

        // Cursor at row 4, not at bottom of region
        grid.move_cursor(4, 2);
        grid.linefeed_in_region(2, 5);

        // Cursor should move down
        assert_eq!(grid.cursor().row, 5);

        // Cursor at bottom of region
        grid.linefeed_in_region(2, 5);

        // Should scroll the region, cursor stays at row 5
        assert_eq!(grid.cursor().row, 5);

        // Row 2 should now have row 3's content
        assert_eq!(grid.row_to_string(2), "33333");
    }

    #[test]
    fn test_erase_in_display_modes() {
        let mut grid = TerminalGrid::with_size(5, 3);

        // Fill grid
        for row in 0..3 {
            grid.move_cursor(row, 0);
            let ch = (b'A' + row as u8) as char;
            for _ in 0..5 {
                grid.put_char(ch);
            }
        }

        // Mode 0: erase from cursor to end
        grid.move_cursor(1, 2);
        grid.erase_in_display(0);
        assert_eq!(grid.row_to_string(0), "AAAAA"); // Unchanged
        assert_eq!(grid.row_to_string(1), "BB   "); // Cleared from col 2
        assert_eq!(grid.row_to_string(2), "     "); // Entire row cleared

        // Reset and test mode 1: erase from start to cursor
        for row in 0..3 {
            grid.move_cursor(row, 0);
            let ch = (b'A' + row as u8) as char;
            for _ in 0..5 {
                grid.put_char(ch);
            }
        }
        grid.move_cursor(1, 2);
        grid.erase_in_display(1);
        assert_eq!(grid.row_to_string(0), "     "); // Entire row cleared
        assert_eq!(grid.row_to_string(1), "   BB"); // Cleared up to col 2 (inclusive)
        assert_eq!(grid.row_to_string(2), "CCCCC"); // Unchanged
    }

    #[test]
    fn test_erase_in_line_modes() {
        let mut grid = TerminalGrid::with_size(5, 3);

        // Fill first row
        grid.move_cursor(0, 0);
        for c in "ABCDE".chars() {
            grid.put_char(c);
        }

        // Mode 0: erase from cursor to end of line
        grid.move_cursor(0, 2);
        grid.erase_in_line(0);
        assert_eq!(grid.row_to_string(0), "AB   ");

        // Reset
        grid.move_cursor(0, 0);
        for c in "ABCDE".chars() {
            grid.put_char(c);
        }

        // Mode 1: erase from start to cursor
        grid.move_cursor(0, 2);
        grid.erase_in_line(1);
        assert_eq!(grid.row_to_string(0), "   DE");

        // Mode 2: erase entire line
        grid.erase_in_line(2);
        assert_eq!(grid.row_to_string(0), "     ");
    }

    // ===== Text Attribute Tests =====

    #[test]
    fn test_cell_with_attributes() {
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        attrs.italic = true;
        attrs.underline = true;
        attrs.blink = true;

        let cell = Cell::with_attributes('X', Color::Indexed(1), Color::Indexed(0), attrs);

        assert_eq!(cell.char, 'X');
        assert_eq!(cell.fg_color, Color::Indexed(1));
        assert_eq!(cell.bg_color, Color::Indexed(0));
        assert!(cell.attributes.bold);
        assert!(cell.attributes.italic);
        assert!(cell.attributes.underline);
        assert!(cell.attributes.blink);
    }

    #[test]
    fn test_put_char_with_bold() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        grid.set_attributes(attrs);

        grid.put_char('B');
        let cell = grid.get_cell(0, 0).unwrap();
        assert!(cell.attributes.bold);
        assert_eq!(cell.char, 'B');
    }

    #[test]
    fn test_put_char_with_italic() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.italic = true;
        grid.set_attributes(attrs);

        grid.put_char('I');
        let cell = grid.get_cell(0, 0).unwrap();
        assert!(cell.attributes.italic);
        assert_eq!(cell.char, 'I');
    }

    #[test]
    fn test_put_char_with_underline() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.underline = true;
        grid.set_attributes(attrs);

        grid.put_char('U');
        let cell = grid.get_cell(0, 0).unwrap();
        assert!(cell.attributes.underline);
        assert_eq!(cell.char, 'U');
    }

    #[test]
    fn test_put_char_with_blink() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.blink = true;
        grid.set_attributes(attrs);

        grid.put_char('B');
        let cell = grid.get_cell(0, 0).unwrap();
        assert!(cell.attributes.blink);
        assert_eq!(cell.char, 'B');
    }

    #[test]
    fn test_put_char_with_all_attributes() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        attrs.italic = true;
        attrs.underline = true;
        attrs.blink = true;
        grid.set_attributes(attrs);

        grid.put_char('A');
        let cell = grid.get_cell(0, 0).unwrap();
        assert!(cell.attributes.bold);
        assert!(cell.attributes.italic);
        assert!(cell.attributes.underline);
        assert!(cell.attributes.blink);
        assert_eq!(cell.char, 'A');
    }

    #[test]
    fn test_attribute_persistence_across_cells() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        attrs.underline = true;
        grid.set_attributes(attrs);

        // Write multiple characters with same attributes
        grid.put_char('A');
        grid.put_char('B');
        grid.put_char('C');

        let cell_a = grid.get_cell(0, 0).unwrap();
        let cell_b = grid.get_cell(0, 1).unwrap();
        let cell_c = grid.get_cell(0, 2).unwrap();

        assert!(cell_a.attributes.bold && cell_a.attributes.underline);
        assert!(cell_b.attributes.bold && cell_b.attributes.underline);
        assert!(cell_c.attributes.bold && cell_c.attributes.underline);
    }

    #[test]
    fn test_change_attributes_between_cells() {
        let mut grid = TerminalGrid::new();

        // First character with bold
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        grid.set_attributes(attrs);
        grid.put_char('A');

        // Second character with underline (not bold)
        attrs.bold = false;
        attrs.underline = true;
        grid.set_attributes(attrs);
        grid.put_char('B');

        // Third character with italic
        attrs.underline = false;
        attrs.italic = true;
        grid.set_attributes(attrs);
        grid.put_char('C');

        let cell_a = grid.get_cell(0, 0).unwrap();
        let cell_b = grid.get_cell(0, 1).unwrap();
        let cell_c = grid.get_cell(0, 2).unwrap();

        assert!(
            cell_a.attributes.bold && !cell_a.attributes.underline && !cell_a.attributes.italic
        );
        assert!(
            !cell_b.attributes.bold && cell_b.attributes.underline && !cell_b.attributes.italic
        );
        assert!(
            !cell_c.attributes.bold && !cell_c.attributes.underline && cell_c.attributes.italic
        );
    }

    #[test]
    fn test_attributes_reset_in_cell() {
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        attrs.italic = true;
        attrs.underline = true;
        attrs.blink = true;

        let mut cell = Cell::with_attributes('X', Color::Default, Color::Default, attrs);
        assert!(!cell.is_empty()); // Has non-default attributes

        cell.reset();
        assert!(cell.is_empty()); // All defaults now
    }

    #[test]
    fn test_grid_attributes_getter() {
        let mut grid = TerminalGrid::new();
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        attrs.italic = true;

        grid.set_attributes(attrs);
        let retrieved = grid.attributes();

        assert!(retrieved.bold);
        assert!(retrieved.italic);
        assert!(!retrieved.underline);
        assert!(!retrieved.blink);
    }

    #[test]
    fn test_cell_attributes_equality() {
        let attrs1 = TextAttributes {
            bold: true,
            italic: false,
            underline: false,
            blink: false,
            ..Default::default()
        };

        let attrs2 = TextAttributes {
            bold: true,
            italic: false,
            underline: false,
            blink: false,
            ..Default::default()
        };

        let attrs3 = TextAttributes {
            bold: false,
            italic: true,
            ..Default::default()
        };

        assert_eq!(attrs1, attrs2);
        assert_ne!(attrs1, attrs3);
    }

    #[test]
    fn test_text_attributes_copy() {
        let mut attrs1 = TextAttributes::default();
        attrs1.bold = true;
        attrs1.italic = true;

        let mut attrs2 = attrs1;

        assert!(attrs2.bold);
        assert!(attrs2.italic);

        // Modify attrs2 - shouldn't affect attrs1
        attrs2.bold = false;
        assert!(attrs1.bold);
        assert!(!attrs2.bold);
    }
}
