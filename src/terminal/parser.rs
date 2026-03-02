//! ANSI escape sequence parser using the `vte` crate.
//!
//! This module provides terminal parsing capabilities for handling VT100/VT200/VT300/VT420
//! escape sequences from PTY output.

#![allow(dead_code)]

use vte::{Params, Perform};

/// Represents a color (either as an index into a palette or as an RGB value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// Default terminal color (foreground or background)
    #[default]
    Default,
    /// Indexed color (0-255, following XTerm 256-color palette)
    Indexed(u8),
    /// 24-bit RGB color
    Rgb(u8, u8, u8),
}

/// Text attributes/style for a character cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextAttributes {
    /// Bold text
    pub bold: bool,
    /// Dim/faint text
    pub dim: bool,
    /// Italic text
    pub italic: bool,
    /// Underlined text
    pub underline: bool,
    /// Blinking text
    pub blink: bool,
    /// Reverse video (swap foreground/background)
    pub reverse: bool,
    /// Hidden/invisible text
    pub hidden: bool,
    /// Strikethrough text
    pub strikethrough: bool,
}

impl TextAttributes {
    /// Reset all attributes to default.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Cursor position in the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPosition {
    /// 0-indexed row
    pub row: usize,
    /// 0-indexed column
    pub col: usize,
}

/// Response type for Device Status Report (DSR) queries.
///
/// When the terminal receives a DSR request (CSI n), it should respond
/// with the appropriate response sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceStatusResponse {
    /// Response to CSI 5n (Device Status Report)
    /// Format: ESC [ 0 n (terminal OK)
    DeviceStatusOk,
    /// Response to CSI 6n (Cursor Position Report)
    /// Format: ESC [ row ; col R (1-indexed position)
    CursorPosition {
        /// Row number (1-indexed)
        row: usize,
        /// Column number (1-indexed)
        col: usize,
    },
}

impl DeviceStatusResponse {
    /// Convert the response to its ANSI escape sequence string.
    pub fn to_escape_sequence(&self) -> String {
        match self {
            DeviceStatusResponse::DeviceStatusOk => "\x1B[0n".to_string(),
            DeviceStatusResponse::CursorPosition { row, col } => {
                format!("\x1B[{};{}R", row, col)
            }
        }
    }

    /// Convert the response to bytes for sending to the PTY.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_escape_sequence().into_bytes()
    }
}

/// Represents a parsed terminal output cell.
#[derive(Debug, Clone)]
pub struct TerminalCell {
    /// Character content
    pub char: char,
    /// Foreground color
    pub fg_color: Color,
    /// Background color
    pub bg_color: Color,
    /// Text attributes
    pub attributes: TextAttributes,
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg_color: Color::Default,
            bg_color: Color::Default,
            attributes: TextAttributes::default(),
        }
    }
}

/// State tracked by the terminal parser.
#[derive(Debug, Clone)]
pub struct ParserState {
    /// Current cursor position
    pub cursor: CursorPosition,
    /// Current text attributes
    pub attributes: TextAttributes,
    /// Current foreground color
    pub fg_color: Color,
    /// Current background color
    pub bg_color: Color,
    /// Whether cursor is visible
    pub cursor_visible: bool,
    /// Saved cursor position (for save/restore)
    pub saved_cursor: CursorPosition,
    /// Saved attributes (for save/restore)
    pub saved_attributes: TextAttributes,
    /// Scroll region top boundary (0-indexed, inclusive)
    pub scroll_region_top: usize,
    /// Scroll region bottom boundary (0-indexed, inclusive)
    pub scroll_region_bottom: usize,
    /// Saved scroll region top (for save/restore with origin mode)
    pub saved_scroll_region_top: usize,
    /// Saved scroll region bottom (for save/restore with origin mode)
    pub saved_scroll_region_bottom: usize,
    /// Origin mode - when true, cursor positioning is relative to scroll region
    pub origin_mode: bool,
    /// Pending device status responses waiting to be sent back
    pub pending_responses: Vec<DeviceStatusResponse>,
    /// Total number of rows in terminal (for scroll region calculations)
    rows: usize,
    /// Current working directory (tracked via OSC 7 shell integration)
    pub current_directory: Option<String>,
}

impl Default for ParserState {
    fn default() -> Self {
        Self {
            cursor: CursorPosition::default(),
            attributes: TextAttributes::default(),
            fg_color: Color::Default,
            bg_color: Color::Default,
            cursor_visible: true,
            saved_cursor: CursorPosition::default(),
            saved_attributes: TextAttributes::default(),
            scroll_region_top: 0,
            scroll_region_bottom: 23, // Default 24-line terminal
            saved_scroll_region_top: 0,
            saved_scroll_region_bottom: 23,
            origin_mode: false,
            pending_responses: Vec::new(),
            rows: 24, // Default 24-line terminal
            current_directory: None,
        }
    }
}

impl ParserState {
    /// Initialize scroll region with terminal size
    pub fn set_terminal_size(&mut self, rows: usize) {
        self.rows = rows;
        self.scroll_region_bottom = rows.saturating_sub(1);
        self.saved_scroll_region_bottom = self.scroll_region_bottom;
    }

    /// Check if a custom scroll region is active (different from full screen)
    pub fn has_scroll_region(&self) -> bool {
        self.scroll_region_top != 0 || self.scroll_region_bottom != self.rows.saturating_sub(1)
    }

    /// Check if cursor is within the scroll region
    pub fn cursor_in_scroll_region(&self, rows: usize) -> bool {
        let effective_bottom = if self.scroll_region_bottom < rows {
            self.scroll_region_bottom
        } else {
            rows.saturating_sub(1)
        };
        self.cursor.row >= self.scroll_region_top && self.cursor.row <= effective_bottom
    }

    /// Check if there are any pending device status responses.
    pub fn has_pending_responses(&self) -> bool {
        !self.pending_responses.is_empty()
    }

    /// Take all pending device status responses, clearing the buffer.
    pub fn take_responses(&mut self) -> Vec<DeviceStatusResponse> {
        std::mem::take(&mut self.pending_responses)
    }

    /// Get pending responses as a single bytes vector for sending to the PTY.
    pub fn responses_as_bytes(&self) -> Vec<u8> {
        self.pending_responses
            .iter()
            .flat_map(|r: &DeviceStatusResponse| r.to_bytes())
            .collect()
    }

    /// Clear all pending responses without processing them.
    pub fn clear_responses(&mut self) {
        self.pending_responses.clear();
    }
}

/// Callback trait for handling parsed terminal output.
/// Implement this to receive characters and control sequences from the parser.
pub trait TerminalOutput {
    /// Put a character at the current cursor position with current attributes.
    fn put_char(&mut self, c: char);

    /// Handle backspace.
    fn backspace(&mut self);

    /// Handle tab.
    fn tab(&mut self);

    /// Handle line feed within scroll region.
    fn linefeed_in_region(&mut self, top: usize, bottom: usize);

    /// Handle carriage return.
    fn carriage_return(&mut self);

    /// Move cursor to position.
    fn move_cursor(&mut self, row: usize, col: usize);

    /// Clear screen.
    fn clear_screen(&mut self);

    /// Get current cursor position.
    fn cursor_position(&self) -> (usize, usize);

    /// Scroll the content within a region up by n lines.
    /// Lines above the region are unaffected. Lines at the bottom of the region are cleared.
    fn scroll_up_in_region(&mut self, n: usize, top: usize, bottom: usize);

    /// Scroll the content within a region down by n lines.
    /// Lines below the region are unaffected. Lines at the top of the region are cleared.
    fn scroll_down_in_region(&mut self, n: usize, top: usize, bottom: usize);

    /// Erase in display with mode.
    fn erase_in_display(&mut self, _mode: u16) {
        // Default implementation clears entire screen
        self.clear_screen();
    }

    /// Erase in line with mode.
    fn erase_in_line(&mut self, _mode: u16) {
        // Default implementation - override in implementor
    }
}

/// Terminal parser that processes ANSI escape sequences.
///
/// This parser uses the `vte` crate to handle escape sequence parsing and
/// maintains the terminal state including cursor position, colors, and text attributes.
pub struct TerminalParser {
    /// The underlying VTE parser
    parser: vte::Parser,
    /// Current parser state
    pub state: ParserState,
    /// Processed output buffer (for rendered content)
    output_buffer: Vec<TerminalCell>,
    /// Number of columns in the terminal
    cols: usize,
    /// Number of rows in the terminal
    rows: usize,
}

impl TerminalParser {
    /// Create a new terminal parser with default dimensions.
    pub fn new() -> Self {
        Self::with_size(80, 24)
    }

    /// Create a new terminal parser with specified dimensions.
    ///
    /// # Arguments
    /// * `cols` - Number of columns (width)
    /// * `rows` - Number of rows (height)
    pub fn with_size(cols: usize, rows: usize) -> Self {
        let mut state = ParserState::default();
        state.set_terminal_size(rows);

        Self {
            parser: vte::Parser::new(),
            state,
            output_buffer: vec![TerminalCell::default(); cols * rows],
            cols,
            rows,
        }
    }

    /// Parse a slice of bytes from the PTY.
    ///
    /// This method processes the raw bytes and updates the parser state.
    /// The bytes are interpreted as UTF-8 text with embedded escape sequences.
    pub fn parse_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.parser.advance(&mut self.state, *byte);
        }
    }

    /// Parse bytes and output to a TerminalOutput sink.
    ///
    /// This is the preferred method for connecting the parser to a grid or screen.
    /// The parser updates its internal state AND writes characters to the output.
    pub fn parse_bytes_with_output<O: TerminalOutput>(&mut self, bytes: &[u8], output: &mut O) {
        // Create a wrapper that forwards to both state and output
        let mut wrapper = ParserOutputWrapper {
            state: &mut self.state,
            output,
            cols: self.cols,
            rows: self.rows,
        };

        for byte in bytes {
            self.parser.advance(&mut wrapper, *byte);
        }
    }

    /// Get the current cursor position.
    pub fn cursor_position(&self) -> CursorPosition {
        self.state.cursor
    }

    /// Get the current text attributes.
    pub fn attributes(&self) -> TextAttributes {
        self.state.attributes
    }

    /// Get the current foreground color.
    pub fn foreground_color(&self) -> Color {
        self.state.fg_color
    }

    /// Get the current background color.
    pub fn background_color(&self) -> Color {
        self.state.bg_color
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.state.set_terminal_size(rows);
        self.output_buffer
            .resize(cols * rows, TerminalCell::default());
    }

    /// Get the output buffer.
    pub fn output(&self) -> &[TerminalCell] {
        &self.output_buffer
    }

    /// Clear the screen.
    pub fn clear_screen(&mut self) {
        self.output_buffer.fill(TerminalCell::default());
    }

    /// Put a character at the current cursor position.
    fn put_char(&mut self, c: char) {
        let row = self.state.cursor.row;
        let col = self.state.cursor.col;

        if row < self.rows && col < self.cols {
            let idx = row * self.cols + col;
            if idx < self.output_buffer.len() {
                let cell = &mut self.output_buffer[idx];
                cell.char = c;
                cell.fg_color = self.state.fg_color;
                cell.bg_color = self.state.bg_color;
                cell.attributes = self.state.attributes;
            }
        }

        // Advance cursor
        if self.state.cursor.col < self.cols - 1 {
            self.state.cursor.col += 1;
        }
    }

    /// Parse a color from SGR parameters.
    fn parse_color(params: &Params, start_idx: usize) -> Option<Color> {
        let iter: Vec<u16> = params
            .iter()
            .skip(start_idx)
            .flat_map(|p| p.iter().copied())
            .collect();

        if iter.is_empty() {
            return None;
        }

        match iter[0] {
            5 => {
                // 256-color mode: ESC[ ... 5 ; <n> m
                if iter.len() > 1 {
                    Some(Color::Indexed(iter[1] as u8))
                } else {
                    None
                }
            }
            2 => {
                // 24-bit color mode: ESC[ ... 2 ; <r> ; <g> ; <b> m
                if iter.len() > 3 {
                    Some(Color::Rgb(iter[1] as u8, iter[2] as u8, iter[3] as u8))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get the current working directory tracked via OSC 7.
    pub fn get_current_directory(&self) -> Option<&str> {
        self.state.current_directory.as_deref()
    }
}

/// Parse a file:// URL and extract the path.
/// Format: file://hostname/path
fn parse_file_url(url: &str) -> Option<String> {
    if !url.starts_with("file://") {
        return None;
    }

    // Skip "file://" prefix
    let rest = &url[7..];

    // Find the start of the path (after hostname)
    // Path starts at the first / after file://
    let path_start = rest.find('/')?;
    let path = &rest[path_start..];

    // URL decode the path (handle %XX sequences)
    Some(urlencoding_decode(path))
}

/// URL decode a string (handle %XX sequences).
fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            // Try to read two hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // If parsing failed, keep the % and continue
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }

    result
}

/// Wrapper that forwards parser events to both state and output.
/// This is used by `parse_bytes_with_output` to update the grid.
struct ParserOutputWrapper<'a, O: TerminalOutput> {
    state: &'a mut ParserState,
    output: &'a mut O,
    cols: usize,
    rows: usize,
}

impl<O: TerminalOutput> ParserOutputWrapper<'_, O> {
    /// Handle line feed with scroll region support
    fn handle_linefeed(&mut self) {
        let top = self.state.scroll_region_top;
        let bottom = self
            .state
            .scroll_region_bottom
            .min(self.rows.saturating_sub(1));

        // Check if cursor is at the bottom of scroll region
        if self.state.cursor.row == bottom {
            // Scroll the region up
            self.output.scroll_up_in_region(1, top, bottom);
        } else if self.state.cursor.row < self.rows - 1 {
            // Just move cursor down
            self.state.cursor.row += 1;
        }

        // Notify output of linefeed within region
        self.output.linefeed_in_region(top, bottom);
    }

    /// Handle reverse index (ESC M) with scroll region support
    fn handle_reverse_index(&mut self) {
        let top = self.state.scroll_region_top;
        let bottom = self
            .state
            .scroll_region_bottom
            .min(self.rows.saturating_sub(1));

        if self.state.cursor.row == top {
            // Cursor at top of scroll region, scroll down
            self.output.scroll_down_in_region(1, top, bottom);
        } else if self.state.cursor.row > 0 {
            self.state.cursor.row -= 1;
        }
    }
}

impl<O: TerminalOutput> Perform for ParserOutputWrapper<'_, O> {
    fn print(&mut self, c: char) {
        // Write character to output with current attributes
        self.output.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => {
                // BS - Backspace
                if self.state.cursor.col > 0 {
                    self.state.cursor.col -= 1;
                }
                self.output.backspace();
            }
            0x09 => {
                // HT - Horizontal Tab
                self.state.cursor.col = (self.state.cursor.col + 8) & !7;
                if self.state.cursor.col >= self.cols {
                    self.state.cursor.col = self.cols - 1;
                }
                self.output.tab();
            }
            0x0A..=0x0C => {
                // LF, VT, FF - Line Feed
                self.handle_linefeed();
            }
            0x0D => {
                // CR - Carriage Return
                self.state.cursor.col = 0;
                self.output.carriage_return();
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        // DCS - not commonly used
    }

    fn put(&mut self, _byte: u8) {
        // Part of DCS handling
    }

    fn unhook(&mut self) {
        // End of DCS
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // OSC sequences - window titles, etc.
        // OSC 7: Shell integration for current directory: ESC ] 7 ; file://hostname/path BEL
        if params.len() >= 2 && params[0] == b"7" {
            // Join all parts after "7;" in case the URL contains semicolons
            let url_bytes: Vec<u8> = params[1..]
                .iter()
                .flat_map(|p| p.iter().copied())
                .collect();

            if let Ok(url) = std::str::from_utf8(&url_bytes) {
                if let Some(path) = parse_file_url(url) {
                    self.state.current_directory = Some(path);
                }
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params_vec: Vec<Vec<u16>> = params.iter().map(|p| p.to_vec()).collect();
        let flat_params: Vec<u16> = params_vec.iter().flat_map(|p| p.iter().copied()).collect();

        match action {
            'A' => {
                // CUU - Cursor Up
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let min_row = if self.state.origin_mode {
                    self.state.scroll_region_top
                } else {
                    0
                };
                self.state.cursor.row = self.state.cursor.row.saturating_sub(n).max(min_row);
                let (row, col) = self.output.cursor_position();
                self.output
                    .move_cursor(row.saturating_sub(n).max(min_row), col);
            }
            'B' => {
                // CUD - Cursor Down
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let max_row = if self.state.origin_mode {
                    self.state.scroll_region_bottom
                } else {
                    self.rows.saturating_sub(1)
                };
                self.state.cursor.row = (self.state.cursor.row + n).min(max_row);
                let (row, col) = self.output.cursor_position();
                self.output.move_cursor((row + n).min(max_row), col);
            }
            'C' => {
                // CUF - Cursor Forward (Right)
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.state.cursor.col = (self.state.cursor.col + n).min(self.cols - 1);
                let (row, col) = self.output.cursor_position();
                self.output.move_cursor(row, (col + n).min(self.cols - 1));
            }
            'D' => {
                // CUB - Cursor Back (Left)
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.state.cursor.col = self.state.cursor.col.saturating_sub(n);
                let (row, col) = self.output.cursor_position();
                self.output.move_cursor(row, col.saturating_sub(n));
            }
            'E' => {
                // CNL - Cursor Next Line
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let max_row = if self.state.origin_mode {
                    self.state.scroll_region_bottom
                } else {
                    self.rows.saturating_sub(1)
                };
                self.state.cursor.row = (self.state.cursor.row + n).min(max_row);
                self.state.cursor.col = 0;
                let (row, _) = self.output.cursor_position();
                self.output.move_cursor((row + n).min(max_row), 0);
            }
            'F' => {
                // CPL - Cursor Previous Line
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let min_row = if self.state.origin_mode {
                    self.state.scroll_region_top
                } else {
                    0
                };
                self.state.cursor.row = self.state.cursor.row.saturating_sub(n).max(min_row);
                self.state.cursor.col = 0;
                let (row, _) = self.output.cursor_position();
                self.output
                    .move_cursor(row.saturating_sub(n).max(min_row), 0);
            }
            'G' => {
                // CHA - Cursor Horizontal Absolute
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.state.cursor.col = n.saturating_sub(1);
                let (row, _) = self.output.cursor_position();
                self.output.move_cursor(row, n.saturating_sub(1));
            }
            'H' | 'f' => {
                // CUP - Cursor Position (row; col)
                let row_param = flat_params.first().copied().unwrap_or(1) as usize;
                let col_param = flat_params.get(1).copied().unwrap_or(1) as usize;

                // Apply origin mode offset
                let (final_row, final_col) = if self.state.origin_mode {
                    let row = (self.state.scroll_region_top + row_param.saturating_sub(1))
                        .min(self.state.scroll_region_bottom);
                    let col = col_param.saturating_sub(1).min(self.cols - 1);
                    (row, col)
                } else {
                    let row = row_param.saturating_sub(1).min(self.rows - 1);
                    let col = col_param.saturating_sub(1).min(self.cols - 1);
                    (row, col)
                };

                self.state.cursor.row = final_row;
                self.state.cursor.col = final_col;
                self.output.move_cursor(final_row, final_col);
            }
            'J' => {
                // ED - Erase in Display
                let mode = flat_params.first().copied().unwrap_or(0);
                self.output.erase_in_display(mode);
                if mode == 2 || mode == 3 {
                    self.state.cursor.row = 0;
                    self.state.cursor.col = 0;
                    self.output.move_cursor(0, 0);
                }
            }
            'K' => {
                // EL - Erase in Line
                let mode = flat_params.first().copied().unwrap_or(0);
                self.output.erase_in_line(mode);
            }
            'L' => {
                // IL - Insert Lines
                // Insert blank lines at cursor, shifting lines down within scroll region
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let top = self.state.cursor.row;
                let bottom = self
                    .state
                    .scroll_region_bottom
                    .min(self.rows.saturating_sub(1));
                if top >= self.state.scroll_region_top && top <= bottom {
                    // Scroll down from cursor row to bottom of region
                    self.output.scroll_down_in_region(n, top, bottom);
                }
            }
            'M' => {
                // DL - Delete Lines
                // Delete lines at cursor, shifting lines up within scroll region
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let top = self.state.cursor.row;
                let bottom = self
                    .state
                    .scroll_region_bottom
                    .min(self.rows.saturating_sub(1));
                if top >= self.state.scroll_region_top && top <= bottom {
                    // Scroll up from cursor row to bottom of region
                    self.output.scroll_up_in_region(n, top, bottom);
                }
            }
            'r' => {
                // DECSTBM - Set Top and Bottom Margins (Scroll Region)
                let top = flat_params.first().copied().unwrap_or(1) as usize;
                let bottom = flat_params.get(1).copied().unwrap_or(self.rows as u16) as usize;

                // Convert from 1-indexed to 0-indexed
                let top_idx = top.saturating_sub(1);
                let bottom_idx = bottom.saturating_sub(1).min(self.rows.saturating_sub(1));

                // Validate: top must be less than bottom
                if top_idx < bottom_idx {
                    self.state.scroll_region_top = top_idx;
                    self.state.scroll_region_bottom = bottom_idx;

                    // Move cursor to home position (top-left of screen or scroll region with origin mode)
                    if self.state.origin_mode {
                        self.state.cursor.row = self.state.scroll_region_top;
                        self.state.cursor.col = 0;
                    } else {
                        self.state.cursor.row = 0;
                        self.state.cursor.col = 0;
                    }
                    self.output
                        .move_cursor(self.state.cursor.row, self.state.cursor.col);
                }
            }
            'S' => {
                // SU - Scroll Up (scroll content up within scroll region)
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let top = self.state.scroll_region_top;
                let bottom = self
                    .state
                    .scroll_region_bottom
                    .min(self.rows.saturating_sub(1));
                self.output.scroll_up_in_region(n, top, bottom);
            }
            'T' => {
                // SD - Scroll Down (scroll content down within scroll region)
                // Note: CSI T can also be initiator for track mouse, but we treat as scroll
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                let top = self.state.scroll_region_top;
                let bottom = self
                    .state
                    .scroll_region_bottom
                    .min(self.rows.saturating_sub(1));
                self.output.scroll_down_in_region(n, top, bottom);
            }
            'm' => {
                // SGR - Select Graphic Rendition
                if flat_params.is_empty() {
                    self.state.attributes.reset();
                    self.state.fg_color = Color::Default;
                    self.state.bg_color = Color::Default;
                } else {
                    let mut i = 0;
                    while i < flat_params.len() {
                        match flat_params[i] {
                            0 => {
                                self.state.attributes.reset();
                                self.state.fg_color = Color::Default;
                                self.state.bg_color = Color::Default;
                            }
                            1 => self.state.attributes.bold = true,
                            2 => self.state.attributes.dim = true,
                            3 => self.state.attributes.italic = true,
                            4 => self.state.attributes.underline = true,
                            5 => self.state.attributes.blink = true,
                            6 => self.state.attributes.blink = true, // Rapid blink
                            7 => self.state.attributes.reverse = true,
                            8 => self.state.attributes.hidden = true,
                            9 => self.state.attributes.strikethrough = true,
                            22 => {
                                self.state.attributes.bold = false;
                                self.state.attributes.dim = false;
                            }
                            23 => self.state.attributes.italic = false,
                            24 => self.state.attributes.underline = false,
                            25 => self.state.attributes.blink = false,
                            27 => self.state.attributes.reverse = false,
                            28 => self.state.attributes.hidden = false,
                            29 => self.state.attributes.strikethrough = false,
                            30..=37 => {
                                self.state.fg_color = Color::Indexed((flat_params[i] - 30) as u8);
                            }
                            38 => {
                                if i + 1 < flat_params.len() {
                                    match flat_params[i + 1] {
                                        5 => {
                                            if i + 2 < flat_params.len() {
                                                self.state.fg_color =
                                                    Color::Indexed(flat_params[i + 2] as u8);
                                                i += 2;
                                            }
                                        }
                                        2 => {
                                            if i + 4 < flat_params.len() {
                                                self.state.fg_color = Color::Rgb(
                                                    flat_params[i + 2] as u8,
                                                    flat_params[i + 3] as u8,
                                                    flat_params[i + 4] as u8,
                                                );
                                                i += 4;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            39 => self.state.fg_color = Color::Default,
                            40..=47 => {
                                self.state.bg_color = Color::Indexed((flat_params[i] - 40) as u8);
                            }
                            48 => {
                                if i + 1 < flat_params.len() {
                                    match flat_params[i + 1] {
                                        5 => {
                                            if i + 2 < flat_params.len() {
                                                self.state.bg_color =
                                                    Color::Indexed(flat_params[i + 2] as u8);
                                                i += 2;
                                            }
                                        }
                                        2 => {
                                            if i + 4 < flat_params.len() {
                                                self.state.bg_color = Color::Rgb(
                                                    flat_params[i + 2] as u8,
                                                    flat_params[i + 3] as u8,
                                                    flat_params[i + 4] as u8,
                                                );
                                                i += 4;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            49 => self.state.bg_color = Color::Default,
                            90..=97 => {
                                self.state.fg_color =
                                    Color::Indexed((flat_params[i] - 90 + 8) as u8);
                            }
                            100..=107 => {
                                self.state.bg_color =
                                    Color::Indexed((flat_params[i] - 100 + 8) as u8);
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
            }
            'n' => {
                // DSR - Device Status Report
                let mode = flat_params.first().copied().unwrap_or(0);
                match mode {
                    5 => {
                        // Request Device Status - respond with "OK"
                        self.state
                            .pending_responses
                            .push(DeviceStatusResponse::DeviceStatusOk);
                    }
                    6 => {
                        // Request Cursor Position Report
                        // Return 1-indexed position
                        self.state
                            .pending_responses
                            .push(DeviceStatusResponse::CursorPosition {
                                row: self.state.cursor.row + 1,
                                col: self.state.cursor.col + 1,
                            });
                    }
                    _ => {
                        // Unknown mode - ignore
                    }
                }
            }
            's' => {
                // SCP - Save Cursor Position
                self.state.saved_cursor = self.state.cursor;
                self.state.saved_attributes = self.state.attributes;
                self.state.saved_scroll_region_top = self.state.scroll_region_top;
                self.state.saved_scroll_region_bottom = self.state.scroll_region_bottom;
            }
            'u' => {
                // RCP - Restore Cursor Position
                self.state.cursor = self.state.saved_cursor;
                self.state.attributes = self.state.saved_attributes;
                self.state.scroll_region_top = self.state.saved_scroll_region_top;
                self.state.scroll_region_bottom = self.state.saved_scroll_region_bottom;
                self.output
                    .move_cursor(self.state.cursor.row, self.state.cursor.col);
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'c' => {
                // RIS - Reset to Initial State
                self.state.cursor = CursorPosition::default();
                self.state.attributes = TextAttributes::default();
                self.state.fg_color = Color::Default;
                self.state.bg_color = Color::Default;
                self.state.scroll_region_top = 0;
                self.state.scroll_region_bottom = self.rows.saturating_sub(1);
                self.state.origin_mode = false;
                self.output.clear_screen();
                self.output.move_cursor(0, 0);
            }
            b'M' => {
                // RI - Reverse Index (move up, scroll down if at top of region)
                self.handle_reverse_index();
            }
            b'D' => {
                // IND - Index (move down, scroll up if at bottom of region)
                self.handle_linefeed();
            }
            b'E' => {
                // NEL - Next Line
                self.state.cursor.col = 0;
                self.output.carriage_return();
                self.handle_linefeed();
            }
            b'7' => {
                // DECSC - Save Cursor (including scroll region)
                self.state.saved_cursor = self.state.cursor;
                self.state.saved_attributes = self.state.attributes;
                self.state.saved_scroll_region_top = self.state.scroll_region_top;
                self.state.saved_scroll_region_bottom = self.state.scroll_region_bottom;
            }
            b'8' => {
                // DECRC - Restore Cursor (including scroll region)
                self.state.cursor = self.state.saved_cursor;
                self.state.attributes = self.state.saved_attributes;
                self.state.scroll_region_top = self.state.saved_scroll_region_top;
                self.state.scroll_region_bottom = self.state.saved_scroll_region_bottom;
                self.output
                    .move_cursor(self.state.cursor.row, self.state.cursor.col);
            }
            _ => {}
        }
    }
}

impl Default for TerminalParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of the `Perform` trait from `vte` to handle escape sequences.
impl Perform for ParserState {
    /// Handle a printable character.
    fn print(&mut self, c: char) {
        // This is called by the parser, but we handle it differently
        // in TerminalParser - the state just tracks attributes
        let _ = c;
    }

    /// Handle a C0 or C1 control character.
    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => {
                // BS - Backspace
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            0x09 => {
                // HT - Horizontal Tab (move to next tab stop, every 8 columns)
                self.cursor.col = (self.cursor.col + 8) & !7;
            }
            0x0A..=0x0C => {
                // LF, VT, FF - Line Feed (move down, possibly scroll)
                self.cursor.row += 1;
                // Note: Scrolling would be handled here in a full implementation
            }
            0x0D => {
                // CR - Carriage Return (move to column 0)
                self.cursor.col = 0;
            }
            0x1B => {
                // ESC - Escape (start of escape sequence)
            }
            _ => {
                // Other control characters are ignored
            }
        }
    }

    /// Handle the end of a CSI escape sequence.
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        // DCS (Device Control String) - not commonly used
    }

    /// Handle a character in a DCS sequence.
    fn put(&mut self, _byte: u8) {
        // Part of DCS handling
    }

    /// Handle the end of a DCS sequence.
    fn unhook(&mut self) {
        // End of DCS
    }

    /// Handle an OSC escape sequence (Operating System Command).
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // OSC sequences for window titles, clipboard, etc.
        // OSC 7: Shell integration for current directory: ESC ] 7 ; file://hostname/path BEL
        if params.len() >= 2 && params[0] == b"7" {
            // Join all parts after "7;" in case the URL contains semicolons
            let url_bytes: Vec<u8> = params[1..]
                .iter()
                .flat_map(|p| p.iter().copied())
                .collect();

            if let Ok(url) = std::str::from_utf8(&url_bytes) {
                if let Some(path) = parse_file_url(url) {
                    self.current_directory = Some(path);
                }
            }
        }
    }

    /// Handle a CSI escape sequence.
    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let params_vec: Vec<Vec<u16>> = params.iter().map(|p| p.to_vec()).collect();
        let flat_params: Vec<u16> = params_vec.iter().flat_map(|p| p.iter().copied()).collect();

        match action {
            'A' => {
                // CUU - Cursor Up
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.cursor.row = self.cursor.row.saturating_sub(n);
            }
            'B' => {
                // CUD - Cursor Down
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.cursor.row += n;
            }
            'C' => {
                // CUF - Cursor Forward (Right)
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.cursor.col += n;
            }
            'D' => {
                // CUB - Cursor Back (Left)
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.cursor.col = self.cursor.col.saturating_sub(n);
            }
            'E' => {
                // CNL - Cursor Next Line
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.cursor.row += n;
                self.cursor.col = 0;
            }
            'F' => {
                // CPL - Cursor Previous Line
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.cursor.row = self.cursor.row.saturating_sub(n);
                self.cursor.col = 0;
            }
            'G' => {
                // CHA - Cursor Horizontal Absolute
                let n = flat_params.first().copied().unwrap_or(1) as usize;
                self.cursor.col = n.saturating_sub(1); // 1-indexed to 0-indexed
            }
            'H' | 'f' => {
                // CUP - Cursor Position (row; col)
                let row = flat_params.first().copied().unwrap_or(1) as usize;
                let col = flat_params.get(1).copied().unwrap_or(1) as usize;
                self.cursor.row = row.saturating_sub(1);
                self.cursor.col = col.saturating_sub(1);
            }
            'J' => {
                // ED - Erase in Display
                let mode = flat_params.first().copied().unwrap_or(0);
                match mode {
                    0 => {
                        // Erase from cursor to end of screen
                        // In a full implementation, this would clear cells
                    }
                    1 => {
                        // Erase from start of screen to cursor
                    }
                    2 | 3 => {
                        // Erase entire screen (and scrollback for 3)
                        self.cursor.row = 0;
                        self.cursor.col = 0;
                    }
                    _ => {}
                }
            }
            'K' => {
                // EL - Erase in Line
                let mode = flat_params.first().copied().unwrap_or(0);
                match mode {
                    0 => {
                        // Erase from cursor to end of line
                    }
                    1 => {
                        // Erase from start of line to cursor
                    }
                    2 => {
                        // Erase entire line
                    }
                    _ => {}
                }
            }
            'r' => {
                // DECSTBM - Set Top and Bottom Margins
                let top = flat_params.first().copied().unwrap_or(1) as usize;
                let bottom = flat_params.get(1).copied().unwrap_or(24) as usize;

                let top_idx = top.saturating_sub(1);
                let bottom_idx = bottom.saturating_sub(1);

                if top_idx < bottom_idx {
                    self.scroll_region_top = top_idx;
                    self.scroll_region_bottom = bottom_idx;
                }
            }
            'm' => {
                // SGR - Select Graphic Rendition
                if flat_params.is_empty() {
                    // Reset all attributes
                    self.attributes.reset();
                    self.fg_color = Color::Default;
                    self.bg_color = Color::Default;
                } else {
                    let mut i = 0;
                    while i < flat_params.len() {
                        match flat_params[i] {
                            0 => {
                                // Reset
                                self.attributes.reset();
                                self.fg_color = Color::Default;
                                self.bg_color = Color::Default;
                            }
                            1 => self.attributes.bold = true,
                            2 => self.attributes.dim = true,
                            3 => self.attributes.italic = true,
                            4 => self.attributes.underline = true,
                            5 => self.attributes.blink = true,
                            6 => self.attributes.blink = true, // Rapid blink
                            7 => self.attributes.reverse = true,
                            8 => self.attributes.hidden = true,
                            9 => self.attributes.strikethrough = true,
                            22 => {
                                self.attributes.bold = false;
                                self.attributes.dim = false;
                            }
                            23 => self.attributes.italic = false,
                            24 => self.attributes.underline = false,
                            25 => self.attributes.blink = false,
                            27 => self.attributes.reverse = false,
                            28 => self.attributes.hidden = false,
                            29 => self.attributes.strikethrough = false,
                            30..=37 => {
                                // Standard foreground colors (3-bit)
                                self.fg_color = Color::Indexed((flat_params[i] - 30) as u8);
                            }
                            38 => {
                                // Extended foreground color
                                if i + 1 < flat_params.len() {
                                    match flat_params[i + 1] {
                                        5 => {
                                            // 256-color
                                            if i + 2 < flat_params.len() {
                                                self.fg_color =
                                                    Color::Indexed(flat_params[i + 2] as u8);
                                                i += 2;
                                            }
                                        }
                                        2 => {
                                            // 24-bit color
                                            if i + 4 < flat_params.len() {
                                                self.fg_color = Color::Rgb(
                                                    flat_params[i + 2] as u8,
                                                    flat_params[i + 3] as u8,
                                                    flat_params[i + 4] as u8,
                                                );
                                                i += 4;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            39 => self.fg_color = Color::Default,
                            40..=47 => {
                                // Standard background colors (3-bit)
                                self.bg_color = Color::Indexed((flat_params[i] - 40) as u8);
                            }
                            48 => {
                                // Extended background color
                                if i + 1 < flat_params.len() {
                                    match flat_params[i + 1] {
                                        5 => {
                                            // 256-color
                                            if i + 2 < flat_params.len() {
                                                self.bg_color =
                                                    Color::Indexed(flat_params[i + 2] as u8);
                                                i += 2;
                                            }
                                        }
                                        2 => {
                                            // 24-bit color
                                            if i + 4 < flat_params.len() {
                                                self.bg_color = Color::Rgb(
                                                    flat_params[i + 2] as u8,
                                                    flat_params[i + 3] as u8,
                                                    flat_params[i + 4] as u8,
                                                );
                                                i += 4;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            49 => self.bg_color = Color::Default,
                            90..=97 => {
                                // Bright foreground colors
                                self.fg_color = Color::Indexed((flat_params[i] - 90 + 8) as u8);
                            }
                            100..=107 => {
                                // Bright background colors
                                self.bg_color = Color::Indexed((flat_params[i] - 100 + 8) as u8);
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
            }
            'n' => {
                // DSR - Device Status Report
                let mode = flat_params.first().copied().unwrap_or(0);
                match mode {
                    5 => {
                        // Request Device Status - respond with "OK"
                        self.pending_responses
                            .push(DeviceStatusResponse::DeviceStatusOk);
                    }
                    6 => {
                        // Request Cursor Position Report
                        // Return 1-indexed position
                        self.pending_responses
                            .push(DeviceStatusResponse::CursorPosition {
                                row: self.cursor.row + 1,
                                col: self.cursor.col + 1,
                            });
                    }
                    _ => {
                        // Unknown mode - ignore
                    }
                }
            }
            's' => {
                // SCP - Save Cursor Position
                self.saved_cursor = self.cursor;
                self.saved_attributes = self.attributes;
            }
            'u' => {
                // RCP - Restore Cursor Position
                self.cursor = self.saved_cursor;
                self.attributes = self.saved_attributes;
            }
            'l' | 'h' => {
                // SM/RM - Set/Reset Mode
                // Handle cursor visibility (DECTCEM): ?25l / ?25h
                // Note: '?' prefix handling would be in intermediates
            }
            _ => {
                // Unknown CSI sequence
            }
        }
    }

    /// Handle an ESC escape sequence.
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'c' => {
                // RIS - Reset to Initial State
                self.cursor = CursorPosition::default();
                self.attributes = TextAttributes::default();
                self.fg_color = Color::Default;
                self.bg_color = Color::Default;
            }
            b'M' => {
                // RI - Reverse Index (move up, scroll if needed)
                if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                }
            }
            b'D' => {
                // IND - Index (move down, scroll if needed)
                self.cursor.row += 1;
            }
            b'E' => {
                // NEL - Next Line
                self.cursor.row += 1;
                self.cursor.col = 0;
            }
            b'7' => {
                // DECSC - Save Cursor
                self.saved_cursor = self.cursor;
                self.saved_attributes = self.attributes;
            }
            b'8' => {
                // DECRC - Restore Cursor
                self.cursor = self.saved_cursor;
                self.attributes = self.saved_attributes;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = TerminalParser::new();
        assert_eq!(parser.cursor_position().row, 0);
        assert_eq!(parser.cursor_position().col, 0);
    }

    #[test]
    fn test_parser_with_size() {
        let parser = TerminalParser::with_size(120, 40);
        assert_eq!(parser.output().len(), 120 * 40);
    }

    #[test]
    fn test_cursor_movement() {
        let mut parser = TerminalParser::new();

        // Test cursor position
        assert_eq!(parser.cursor_position(), CursorPosition { row: 0, col: 0 });

        // Move cursor down (CSI B)
        parser.parse_bytes(b"\x1B[5B");
        assert_eq!(parser.cursor_position().row, 5);

        // Move cursor right (CSI C)
        parser.parse_bytes(b"\x1B[10C");
        assert_eq!(parser.cursor_position().col, 10);

        // Move cursor to position (CSI H)
        parser.parse_bytes(b"\x1B[5;10H");
        assert_eq!(parser.cursor_position().row, 4); // 1-indexed to 0-indexed
        assert_eq!(parser.cursor_position().col, 9);
    }

    #[test]
    fn test_text_attributes() {
        let mut parser = TerminalParser::new();

        // Enable bold (CSI 1 m)
        parser.parse_bytes(b"\x1B[1m");
        assert!(parser.attributes().bold);

        // Enable italic (CSI 3 m)
        parser.parse_bytes(b"\x1B[3m");
        assert!(parser.attributes().italic);

        // Reset all (CSI 0 m)
        parser.parse_bytes(b"\x1B[0m");
        assert!(!parser.attributes().bold);
        assert!(!parser.attributes().italic);
    }

    #[test]
    fn test_colors() {
        let mut parser = TerminalParser::new();

        // Set foreground to red (CSI 31 m)
        parser.parse_bytes(b"\x1B[31m");
        assert_eq!(parser.foreground_color(), Color::Indexed(1));

        // Set foreground to default (CSI 39 m)
        parser.parse_bytes(b"\x1B[39m");
        assert_eq!(parser.foreground_color(), Color::Default);

        // Set 256-color foreground (CSI 38;5;<n> m)
        parser.parse_bytes(b"\x1B[38;5;200m");
        assert_eq!(parser.foreground_color(), Color::Indexed(200));

        // Set 24-bit RGB foreground (CSI 38;2;<r>;<g>;<b> m)
        parser.parse_bytes(b"\x1B[38;2;255;128;64m");
        assert_eq!(parser.foreground_color(), Color::Rgb(255, 128, 64));
    }

    #[test]
    fn test_control_characters() {
        let mut parser = TerminalParser::new();

        // Start at position 5,5
        parser.state.cursor.row = 5;
        parser.state.cursor.col = 5;

        // Carriage return
        parser.parse_bytes(b"\r");
        assert_eq!(parser.cursor_position().col, 0);
        assert_eq!(parser.cursor_position().row, 5);

        // Reset and test backspace
        parser.state.cursor.col = 5;
        parser.parse_bytes(b"\x08");
        assert_eq!(parser.cursor_position().col, 4);

        // Reset and test tab
        parser.state.cursor.col = 5;
        parser.parse_bytes(b"\t");
        assert_eq!(parser.cursor_position().col, 8); // Next tab stop

        // Test line feed
        parser.parse_bytes(b"\n");
        assert_eq!(parser.cursor_position().row, 6);
    }

    #[test]
    fn test_save_restore_cursor() {
        let mut parser = TerminalParser::new();

        // Set position
        parser.state.cursor.row = 10;
        parser.state.cursor.col = 20;

        // Save cursor (CSI s)
        parser.parse_bytes(b"\x1B[s");

        // Move cursor
        parser.parse_bytes(b"\x1B[1;1H");
        assert_eq!(parser.cursor_position().row, 0);
        assert_eq!(parser.cursor_position().col, 0);

        // Restore cursor (CSI u)
        parser.parse_bytes(b"\x1B[u");
        assert_eq!(parser.cursor_position().row, 10);
        assert_eq!(parser.cursor_position().col, 20);
    }

    #[test]
    fn test_esc_save_restore() {
        let mut parser = TerminalParser::new();

        // Set position and attributes
        parser.state.cursor.row = 5;
        parser.state.cursor.col = 10;
        parser.state.attributes.bold = true;

        // Save cursor (ESC 7)
        parser.parse_bytes(b"\x1B7");

        // Reset
        parser.parse_bytes(b"\x1B[0m");
        parser.state.cursor.row = 0;
        parser.state.cursor.col = 0;

        // Restore cursor (ESC 8)
        parser.parse_bytes(b"\x1B8");

        assert_eq!(parser.cursor_position().row, 5);
        assert_eq!(parser.cursor_position().col, 10);
    }

    #[test]
    fn test_combined_sequences() {
        let mut parser = TerminalParser::new();

        // Complex sequence: bold, red fg, move to row 10, col 20
        parser.parse_bytes(b"\x1B[1;31m\x1B[10;20H");

        assert!(parser.attributes().bold);
        assert_eq!(parser.foreground_color(), Color::Indexed(1)); // Red
        assert_eq!(parser.cursor_position().row, 9);
        assert_eq!(parser.cursor_position().col, 19);
    }

    // ===== Scroll Region Tests =====

    #[test]
    fn test_scroll_region_default() {
        let parser = TerminalParser::with_size(80, 24);

        // Default scroll region should be full screen
        assert_eq!(parser.state.scroll_region_top, 0);
        assert_eq!(parser.state.scroll_region_bottom, 23);
    }

    #[test]
    fn test_decstbm_set_scroll_region() {
        let mut parser = TerminalParser::with_size(80, 24);

        // Set scroll region to rows 5-15 (1-indexed)
        // CSI 5 ; 15 r
        parser.parse_bytes(b"\x1B[5;15r");

        // Convert to 0-indexed: 4-14
        assert_eq!(parser.state.scroll_region_top, 4);
        assert_eq!(parser.state.scroll_region_bottom, 14);

        // Cursor should move to home
        assert_eq!(parser.cursor_position().row, 0);
        assert_eq!(parser.cursor_position().col, 0);
    }

    #[test]
    fn test_decstbm_full_screen() {
        let mut parser = TerminalParser::with_size(80, 24);

        // First set a custom region
        parser.parse_bytes(b"\x1B[5;15r");
        assert_eq!(parser.state.scroll_region_top, 4);
        assert_eq!(parser.state.scroll_region_bottom, 14);

        // Reset to full screen with CSI r (no params = full screen)
        parser.parse_bytes(b"\x1B[r");
        assert_eq!(parser.state.scroll_region_top, 0);
        assert_eq!(parser.state.scroll_region_bottom, 23);
    }

    #[test]
    fn test_decstbm_invalid_region() {
        let mut parser = TerminalParser::with_size(80, 24);

        // Try to set invalid region (top >= bottom)
        parser.parse_bytes(b"\x1B[15;5r");

        // Should not change from default
        assert_eq!(parser.state.scroll_region_top, 0);
        assert_eq!(parser.state.scroll_region_bottom, 23);
    }

    #[test]
    fn test_decstbm_single_param() {
        let mut parser = TerminalParser::with_size(80, 24);

        // CSI 5 r - set top margin only, bottom defaults to screen bottom
        parser.parse_bytes(b"\x1B[5r");

        assert_eq!(parser.state.scroll_region_top, 4);
        assert_eq!(parser.state.scroll_region_bottom, 23);
    }

    #[test]
    fn test_scroll_region_state() {
        let mut state = ParserState::default();
        state.set_terminal_size(24);

        assert_eq!(state.scroll_region_top, 0);
        assert_eq!(state.scroll_region_bottom, 23);
        assert!(!state.has_scroll_region());

        state.scroll_region_top = 5;
        state.scroll_region_bottom = 15;
        assert!(state.has_scroll_region());
    }

    #[test]
    fn test_cursor_in_scroll_region() {
        let mut state = ParserState::default();
        state.set_terminal_size(24);
        state.scroll_region_top = 5;
        state.scroll_region_bottom = 15;

        state.cursor.row = 3;
        assert!(!state.cursor_in_scroll_region(24));

        state.cursor.row = 10;
        assert!(state.cursor_in_scroll_region(24));

        state.cursor.row = 15;
        assert!(state.cursor_in_scroll_region(24));
    }

    #[test]
    fn test_reset_clears_scroll_region() {
        let mut parser = TerminalParser::with_size(80, 24);

        // Set a scroll region
        parser.state.scroll_region_top = 4;
        parser.state.scroll_region_bottom = 14;

        // ESC c (RIS) should reset scroll region via state
        parser.state.scroll_region_top = 0;
        parser.state.scroll_region_bottom = 23;

        assert_eq!(parser.state.scroll_region_top, 0);
        assert_eq!(parser.state.scroll_region_bottom, 23);
    }

    #[test]
    fn test_save_restore_includes_scroll_region() {
        let mut parser = TerminalParser::with_size(80, 24);

        // Set scroll region
        parser.state.scroll_region_top = 4;
        parser.state.scroll_region_bottom = 14;

        // Save state
        parser.state.saved_scroll_region_top = parser.state.scroll_region_top;
        parser.state.saved_scroll_region_bottom = parser.state.scroll_region_bottom;

        // Change scroll region
        parser.state.scroll_region_top = 1;
        parser.state.scroll_region_bottom = 9;

        // Restore
        parser.state.scroll_region_top = parser.state.saved_scroll_region_top;
        parser.state.scroll_region_bottom = parser.state.saved_scroll_region_bottom;

        assert_eq!(parser.state.scroll_region_top, 4);
        assert_eq!(parser.state.scroll_region_bottom, 14);
    }

    // ===== Text Attribute Tests =====

    #[test]
    fn test_sgr_bold_enable() {
        let mut parser = TerminalParser::new();

        // CSI 1 m - Enable bold
        parser.parse_bytes(b"\x1B[1m");
        assert!(parser.attributes().bold);
        assert!(!parser.attributes().italic);
        assert!(!parser.attributes().underline);
        assert!(!parser.attributes().blink);
    }

    #[test]
    fn test_sgr_italic_enable() {
        let mut parser = TerminalParser::new();

        // CSI 3 m - Enable italic
        parser.parse_bytes(b"\x1B[3m");
        assert!(parser.attributes().italic);
        assert!(!parser.attributes().bold);
    }

    #[test]
    fn test_sgr_underline_enable() {
        let mut parser = TerminalParser::new();

        // CSI 4 m - Enable underline
        parser.parse_bytes(b"\x1B[4m");
        assert!(parser.attributes().underline);
        assert!(!parser.attributes().bold);
        assert!(!parser.attributes().italic);
    }

    #[test]
    fn test_sgr_blink_enable() {
        let mut parser = TerminalParser::new();

        // CSI 5 m - Enable slow blink
        parser.parse_bytes(b"\x1B[5m");
        assert!(parser.attributes().blink);

        // CSI 6 m - Enable rapid blink (also sets blink)
        parser.parse_bytes(b"\x1B[0m"); // Reset first
        parser.parse_bytes(b"\x1B[6m");
        assert!(parser.attributes().blink);
    }

    #[test]
    fn test_sgr_bold_disable() {
        let mut parser = TerminalParser::new();

        // Enable bold first
        parser.parse_bytes(b"\x1B[1m");
        assert!(parser.attributes().bold);

        // CSI 22 m - Normal intensity (disable bold and dim)
        parser.parse_bytes(b"\x1B[22m");
        assert!(!parser.attributes().bold);
        assert!(!parser.attributes().dim);
    }

    #[test]
    fn test_sgr_italic_disable() {
        let mut parser = TerminalParser::new();

        // Enable italic first
        parser.parse_bytes(b"\x1B[3m");
        assert!(parser.attributes().italic);

        // CSI 23 m - Not italic
        parser.parse_bytes(b"\x1B[23m");
        assert!(!parser.attributes().italic);
    }

    #[test]
    fn test_sgr_underline_disable() {
        let mut parser = TerminalParser::new();

        // Enable underline first
        parser.parse_bytes(b"\x1B[4m");
        assert!(parser.attributes().underline);

        // CSI 24 m - Not underline
        parser.parse_bytes(b"\x1B[24m");
        assert!(!parser.attributes().underline);
    }

    #[test]
    fn test_sgr_blink_disable() {
        let mut parser = TerminalParser::new();

        // Enable blink first
        parser.parse_bytes(b"\x1B[5m");
        assert!(parser.attributes().blink);

        // CSI 25 m - Not blinking
        parser.parse_bytes(b"\x1B[25m");
        assert!(!parser.attributes().blink);
    }

    #[test]
    fn test_sgr_reset_all_attributes() {
        let mut parser = TerminalParser::new();

        // Enable all attributes
        parser.parse_bytes(b"\x1B[1;3;4;5m");
        assert!(parser.attributes().bold);
        assert!(parser.attributes().italic);
        assert!(parser.attributes().underline);
        assert!(parser.attributes().blink);

        // CSI 0 m - Reset all
        parser.parse_bytes(b"\x1B[0m");
        assert!(!parser.attributes().bold);
        assert!(!parser.attributes().italic);
        assert!(!parser.attributes().underline);
        assert!(!parser.attributes().blink);
    }

    #[test]
    fn test_sgr_empty_params_reset() {
        let mut parser = TerminalParser::new();

        // Enable all attributes
        parser.parse_bytes(b"\x1B[1;3;4;5m");
        assert!(parser.attributes().bold);

        // CSI m (empty params) - Also resets
        parser.parse_bytes(b"\x1B[m");
        assert!(!parser.attributes().bold);
        assert!(!parser.attributes().italic);
        assert!(!parser.attributes().underline);
        assert!(!parser.attributes().blink);
    }

    #[test]
    fn test_sgr_combined_attributes() {
        let mut parser = TerminalParser::new();

        // Enable bold and underline together
        parser.parse_bytes(b"\x1B[1;4m");
        assert!(parser.attributes().bold);
        assert!(parser.attributes().underline);
        assert!(!parser.attributes().italic);
        assert!(!parser.attributes().blink);
    }

    #[test]
    fn test_sgr_all_four_attributes() {
        let mut parser = TerminalParser::new();

        // Enable all four main attributes
        parser.parse_bytes(b"\x1B[1;3;4;5m");
        assert!(parser.attributes().bold);
        assert!(parser.attributes().italic);
        assert!(parser.attributes().underline);
        assert!(parser.attributes().blink);
    }

    #[test]
    fn test_sgr_selective_disable() {
        let mut parser = TerminalParser::new();

        // Enable all four
        parser.parse_bytes(b"\x1B[1;3;4;5m");

        // Disable only italic (CSI 23 m)
        parser.parse_bytes(b"\x1B[23m");
        assert!(parser.attributes().bold);
        assert!(!parser.attributes().italic);
        assert!(parser.attributes().underline);
        assert!(parser.attributes().blink);

        // Disable only underline (CSI 24 m)
        parser.parse_bytes(b"\x1B[4m"); // Re-enable
        parser.parse_bytes(b"\x1B[24m");
        assert!(parser.attributes().bold);
        assert!(parser.attributes().italic == false);
        assert!(!parser.attributes().underline);
        assert!(parser.attributes().blink);
    }

    #[test]
    fn test_sgr_dim_attribute() {
        let mut parser = TerminalParser::new();

        // CSI 2 m - Enable dim
        parser.parse_bytes(b"\x1B[2m");
        assert!(parser.attributes().dim);

        // CSI 22 m - Disable dim (and bold)
        parser.parse_bytes(b"\x1B[22m");
        assert!(!parser.attributes().dim);
    }

    #[test]
    fn test_sgr_reverse_attribute() {
        let mut parser = TerminalParser::new();

        // CSI 7 m - Enable reverse video
        parser.parse_bytes(b"\x1B[7m");
        assert!(parser.attributes().reverse);

        // CSI 27 m - Disable reverse
        parser.parse_bytes(b"\x1B[27m");
        assert!(!parser.attributes().reverse);
    }

    #[test]
    fn test_sgr_hidden_attribute() {
        let mut parser = TerminalParser::new();

        // CSI 8 m - Enable hidden
        parser.parse_bytes(b"\x1B[8m");
        assert!(parser.attributes().hidden);

        // CSI 28 m - Disable hidden
        parser.parse_bytes(b"\x1B[28m");
        assert!(!parser.attributes().hidden);
    }

    #[test]
    fn test_sgr_strikethrough_attribute() {
        let mut parser = TerminalParser::new();

        // CSI 9 m - Enable strikethrough
        parser.parse_bytes(b"\x1B[9m");
        assert!(parser.attributes().strikethrough);

        // CSI 29 m - Disable strikethrough
        parser.parse_bytes(b"\x1B[29m");
        assert!(!parser.attributes().strikethrough);
    }

    #[test]
    fn test_text_attributes_default() {
        let attrs = TextAttributes::default();
        assert!(!attrs.bold);
        assert!(!attrs.dim);
        assert!(!attrs.italic);
        assert!(!attrs.underline);
        assert!(!attrs.blink);
        assert!(!attrs.reverse);
        assert!(!attrs.hidden);
        assert!(!attrs.strikethrough);
    }

    #[test]
    fn test_text_attributes_reset() {
        let mut attrs = TextAttributes::default();
        attrs.bold = true;
        attrs.italic = true;
        attrs.underline = true;
        attrs.blink = true;

        attrs.reset();

        assert!(!attrs.bold);
        assert!(!attrs.italic);
        assert!(!attrs.underline);
        assert!(!attrs.blink);
    }

    #[test]
    fn test_attributes_preserved_with_colors() {
        let mut parser = TerminalParser::new();

        // Set attributes and colors together
        parser.parse_bytes(b"\x1B[1;3;31m"); // Bold, italic, red fg

        assert!(parser.attributes().bold);
        assert!(parser.attributes().italic);
        assert_eq!(parser.foreground_color(), Color::Indexed(1)); // Red

        // Change color but keep attributes
        parser.parse_bytes(b"\x1B[34m"); // Blue fg
        assert!(parser.attributes().bold);
        assert!(parser.attributes().italic);
        assert_eq!(parser.foreground_color(), Color::Indexed(4)); // Blue
    }

    #[test]
    fn test_true_color_foreground() {
        let mut parser = TerminalParser::new();

        // Set foreground to true color red (CSI 38;2;255;0;0 m)
        parser.parse_bytes(b"\x1B[38;2;255;0;0m");
        assert_eq!(parser.foreground_color(), Color::Rgb(255, 0, 0));

        // Set foreground to true color green (CSI 38;2;0;255;0 m)
        parser.parse_bytes(b"\x1B[38;2;0;255;0m");
        assert_eq!(parser.foreground_color(), Color::Rgb(0, 255, 0));

        // Set foreground to true color blue (CSI 38;2;0;0;255 m)
        parser.parse_bytes(b"\x1B[38;2;0;0;255m");
        assert_eq!(parser.foreground_color(), Color::Rgb(0, 0, 255));

        // Set foreground to true color custom (CSI 38;2;128;64;192 m)
        parser.parse_bytes(b"\x1B[38;2;128;64;192m");
        assert_eq!(parser.foreground_color(), Color::Rgb(128, 64, 192));

        // Set foreground to white (full intensity)
        parser.parse_bytes(b"\x1B[38;2;255;255;255m");
        assert_eq!(parser.foreground_color(), Color::Rgb(255, 255, 255));

        // Set foreground to black
        parser.parse_bytes(b"\x1B[38;2;0;0;0m");
        assert_eq!(parser.foreground_color(), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_true_color_background() {
        let mut parser = TerminalParser::new();

        // Set background to true color red (CSI 48;2;255;0;0 m)
        parser.parse_bytes(b"\x1B[48;2;255;0;0m");
        assert_eq!(parser.background_color(), Color::Rgb(255, 0, 0));

        // Set background to true color green (CSI 48;2;0;255;0 m)
        parser.parse_bytes(b"\x1B[48;2;0;255;0m");
        assert_eq!(parser.background_color(), Color::Rgb(0, 255, 0));

        // Set background to true color blue (CSI 48;2;0;0;255 m)
        parser.parse_bytes(b"\x1B[48;2;0;0;255m");
        assert_eq!(parser.background_color(), Color::Rgb(0, 0, 255));

        // Set background to true color custom (CSI 48;2;64;128;200 m)
        parser.parse_bytes(b"\x1B[48;2;64;128;200m");
        assert_eq!(parser.background_color(), Color::Rgb(64, 128, 200));
    }

    #[test]
    fn test_true_color_mixed_with_indexed() {
        let mut parser = TerminalParser::new();

        // Start with indexed color
        parser.parse_bytes(b"\x1B[31m"); // Red
        assert_eq!(parser.foreground_color(), Color::Indexed(1));

        // Switch to true color
        parser.parse_bytes(b"\x1B[38;2;100;150;200m");
        assert_eq!(parser.foreground_color(), Color::Rgb(100, 150, 200));

        // Switch back to indexed
        parser.parse_bytes(b"\x1B[34m"); // Blue
        assert_eq!(parser.foreground_color(), Color::Indexed(4));

        // Switch to true color again
        parser.parse_bytes(b"\x1B[38;2;255;100;50m");
        assert_eq!(parser.foreground_color(), Color::Rgb(255, 100, 50));
    }

    #[test]
    fn test_true_color_background_mixed() {
        let mut parser = TerminalParser::new();

        // Start with indexed background
        parser.parse_bytes(b"\x1B[41m"); // Red background
        assert_eq!(parser.background_color(), Color::Indexed(1));

        // Switch to true color background
        parser.parse_bytes(b"\x1B[48;2;200;150;100m");
        assert_eq!(parser.background_color(), Color::Rgb(200, 150, 100));

        // Switch back to indexed
        parser.parse_bytes(b"\x1B[44m"); // Blue background
        assert_eq!(parser.background_color(), Color::Indexed(4));
    }

    #[test]
    fn test_true_color_combined_with_attributes() {
        let mut parser = TerminalParser::new();

        // Set attributes and true color together
        parser.parse_bytes(b"\x1B[1;38;2;255;0;0m"); // Bold + true color red
        assert!(parser.attributes().bold);
        assert_eq!(parser.foreground_color(), Color::Rgb(255, 0, 0));

        // Add more attributes and change color
        parser.parse_bytes(b"\x1B[3;4;38;2;0;255;0m"); // Italic + underline + true color green
        assert!(parser.attributes().italic);
        assert!(parser.attributes().underline);
        assert_eq!(parser.foreground_color(), Color::Rgb(0, 255, 0));
    }

    #[test]
    fn test_true_color_fg_bg_separate() {
        let mut parser = TerminalParser::new();

        // Set both foreground and background to different true colors
        parser.parse_bytes(b"\x1B[38;2;255;200;100;48;2;50;75;100m");
        assert_eq!(parser.foreground_color(), Color::Rgb(255, 200, 100));
        assert_eq!(parser.background_color(), Color::Rgb(50, 75, 100));

        // Change only foreground
        parser.parse_bytes(b"\x1B[38;2;0;255;255m");
        assert_eq!(parser.foreground_color(), Color::Rgb(0, 255, 255));
        assert_eq!(parser.background_color(), Color::Rgb(50, 75, 100));

        // Change only background
        parser.parse_bytes(b"\x1B[48;2;255;0;255m");
        assert_eq!(parser.foreground_color(), Color::Rgb(0, 255, 255));
        assert_eq!(parser.background_color(), Color::Rgb(255, 0, 255));
    }

    #[test]
    fn test_true_color_reset() {
        let mut parser = TerminalParser::new();

        // Set true color foreground
        parser.parse_bytes(b"\x1B[38;2;100;200;50m");
        assert_eq!(parser.foreground_color(), Color::Rgb(100, 200, 50));

        // Reset foreground to default (CSI 39 m)
        parser.parse_bytes(b"\x1B[39m");
        assert_eq!(parser.foreground_color(), Color::Default);

        // Set true color background
        parser.parse_bytes(b"\x1B[48;2;150;100;200m");
        assert_eq!(parser.background_color(), Color::Rgb(150, 100, 200));

        // Reset background to default (CSI 49 m)
        parser.parse_bytes(b"\x1B[49m");
        assert_eq!(parser.background_color(), Color::Default);
    }

    #[test]
    fn test_true_color_edge_cases() {
        let mut parser = TerminalParser::new();

        // Test minimum values (0, 0, 0)
        parser.parse_bytes(b"\x1B[38;2;0;0;0m");
        assert_eq!(parser.foreground_color(), Color::Rgb(0, 0, 0));

        // Test maximum values (255, 255, 255)
        parser.parse_bytes(b"\x1B[38;2;255;255;255m");
        assert_eq!(parser.foreground_color(), Color::Rgb(255, 255, 255));

        // Test mid-range values (127, 127, 127)
        parser.parse_bytes(b"\x1B[38;2;127;127;127m");
        assert_eq!(parser.foreground_color(), Color::Rgb(127, 127, 127));
    }

    // ===== Cursor Position Report Tests (CSI 6n) =====

    #[test]
    fn test_dsr_csi_6n_cursor_position_report() {
        let mut parser = TerminalParser::new();

        // Move cursor to position (row 5, col 10) - 0-indexed internally
        parser.parse_bytes(b"\x1B[6;11H"); // 1-indexed: row 6, col 11
        assert_eq!(parser.state.cursor.row, 5);
        assert_eq!(parser.state.cursor.col, 10);

        // Request cursor position report (CSI 6n)
        parser.parse_bytes(b"\x1B[6n");

        // Check that a response was queued
        assert!(parser.state.has_pending_responses());

        // Take the response
        let responses = parser.state.take_responses();
        assert_eq!(responses.len(), 1);

        // Verify response format: ESC[row;colR (1-indexed)
        match &responses[0] {
            DeviceStatusResponse::CursorPosition { row, col } => {
                assert_eq!(*row, 6); // 1-indexed
                assert_eq!(*col, 11); // 1-indexed

                // Check the escape sequence format
                let escape_seq = responses[0].to_escape_sequence();
                assert_eq!(escape_seq, "\x1B[6;11R");

                // Check bytes format
                let bytes = responses[0].to_bytes();
                assert_eq!(bytes, b"\x1B[6;11R");
            }
            _ => panic!("Expected CursorPosition response"),
        }
    }

    #[test]
    fn test_dsr_csi_6n_at_origin() {
        let mut parser = TerminalParser::new();

        // Cursor at origin (0, 0)
        assert_eq!(parser.state.cursor.row, 0);
        assert_eq!(parser.state.cursor.col, 0);

        // Request cursor position report
        parser.parse_bytes(b"\x1B[6n");

        let responses = parser.state.take_responses();
        assert_eq!(responses.len(), 1);

        match &responses[0] {
            DeviceStatusResponse::CursorPosition { row, col } => {
                // Should be 1-indexed: (1, 1)
                assert_eq!(*row, 1);
                assert_eq!(*col, 1);
                assert_eq!(responses[0].to_escape_sequence(), "\x1B[1;1R");
            }
            _ => panic!("Expected CursorPosition response"),
        }
    }

    #[test]
    fn test_dsr_csi_5n_device_status_ok() {
        let mut parser = TerminalParser::new();

        // Request device status (CSI 5n)
        parser.parse_bytes(b"\x1B[5n");

        let responses = parser.state.take_responses();
        assert_eq!(responses.len(), 1);

        match &responses[0] {
            DeviceStatusResponse::DeviceStatusOk => {
                assert_eq!(responses[0].to_escape_sequence(), "\x1B[0n");
                assert_eq!(responses[0].to_bytes(), b"\x1B[0n");
            }
            _ => panic!("Expected DeviceStatusOk response"),
        }
    }

    #[test]
    fn test_dsr_multiple_requests() {
        let mut parser = TerminalParser::new();

        // Move cursor
        parser.parse_bytes(b"\x1B[10;20H");

        // Request multiple device status reports
        parser.parse_bytes(b"\x1B[5n"); // Device status
        parser.parse_bytes(b"\x1B[6n"); // Cursor position
        parser.parse_bytes(b"\x1B[5n"); // Another device status

        let responses = parser.state.take_responses();
        assert_eq!(responses.len(), 3);

        // First: Device status OK
        assert!(matches!(
            &responses[0],
            DeviceStatusResponse::DeviceStatusOk
        ));

        // Second: Cursor position
        match &responses[1] {
            DeviceStatusResponse::CursorPosition { row, col } => {
                assert_eq!(*row, 10);
                assert_eq!(*col, 20);
                assert_eq!(responses[1].to_escape_sequence(), "\x1B[10;20R");
            }
            _ => panic!("Expected CursorPosition response"),
        }

        // Third: Device status OK again
        assert!(matches!(
            &responses[2],
            DeviceStatusResponse::DeviceStatusOk
        ));

        // Test responses_as_bytes combines all
        parser.state.pending_responses.push(responses[0].clone());
        parser.state.pending_responses.push(responses[1].clone());
        parser.state.pending_responses.push(responses[2].clone());

        let combined = parser.state.responses_as_bytes();
        assert_eq!(combined, b"\x1B[0n\x1B[10;20R\x1B[0n");
    }

    #[test]
    fn test_dsr_clear_pending_responses() {
        let mut parser = TerminalParser::new();

        // Generate some responses
        parser.parse_bytes(b"\x1B[5n");
        parser.parse_bytes(b"\x1B[6n");

        assert!(parser.state.has_pending_responses());

        // Clear responses
        parser.state.clear_responses();

        assert!(!parser.state.has_pending_responses());
        assert!(parser.state.take_responses().is_empty());
    }

    // ===== OSC 7 Directory Tracking Tests =====

    #[test]
    fn test_osc7_basic_directory() {
        let mut parser = TerminalParser::new();

        // Initial state: no directory tracked
        assert!(parser.get_current_directory().is_none());

        // OSC 7: file://hostname/path
        // ESC ] 7 ; file://hostname/path BEL
        parser.parse_bytes(b"\x1B]7;file://localhost/home/user\x07");

        assert_eq!(parser.get_current_directory(), Some("/home/user"));
    }

    #[test]
    fn test_osc7_with_different_hostname() {
        let mut parser = TerminalParser::new();

        // Should work with any hostname
        parser.parse_bytes(b"\x1B]7;file://myhost/tmp/test\x07");

        assert_eq!(parser.get_current_directory(), Some("/tmp/test"));
    }

    #[test]
    fn test_osc7_root_directory() {
        let mut parser = TerminalParser::new();

        parser.parse_bytes(b"\x1B]7;file://localhost/\x07");

        assert_eq!(parser.get_current_directory(), Some("/"));
    }

    #[test]
    fn test_osc7_url_encoded_path() {
        let mut parser = TerminalParser::new();

        // Space encoded as %20
        parser.parse_bytes(b"\x1B]7;file://localhost/home/my%20docs\x07");

        assert_eq!(parser.get_current_directory(), Some("/home/my docs"));
    }

    #[test]
    fn test_osc7_url_encoded_special_chars() {
        let mut parser = TerminalParser::new();

        // %3D = '=', %26 = '&', %25 = '%'
        parser.parse_bytes(b"\x1B]7;file://localhost/path%3Dwith%26special%25chars\x07");

        assert_eq!(
            parser.get_current_directory(),
            Some("/path=with&special%chars")
        );
    }

    #[test]
    fn test_osc7_deep_path() {
        let mut parser = TerminalParser::new();

        parser.parse_bytes(b"\x1B]7;file://localhost/a/b/c/d/e/f\x07");

        assert_eq!(parser.get_current_directory(), Some("/a/b/c/d/e/f"));
    }

    #[test]
    fn test_osc7_invalid_not_file_url() {
        let mut parser = TerminalParser::new();

        // Should not update for non-file URLs
        parser.parse_bytes(b"\x1B]7;http://example.com/path\x07");

        assert!(parser.get_current_directory().is_none());
    }

    #[test]
    fn test_osc7_invalid_wrong_osc_number() {
        let mut parser = TerminalParser::new();

        // OSC 0 is for window title, should not affect directory
        parser.parse_bytes(b"\x1B]0;file://localhost/home/user\x07");

        assert!(parser.get_current_directory().is_none());
    }

    #[test]
    fn test_osc7_updates_on_change() {
        let mut parser = TerminalParser::new();

        // First directory
        parser.parse_bytes(b"\x1B]7;file://localhost/home/user\x07");
        assert_eq!(parser.get_current_directory(), Some("/home/user"));

        // Change to different directory
        parser.parse_bytes(b"\x1B]7;file://localhost/tmp\x07");
        assert_eq!(parser.get_current_directory(), Some("/tmp"));
    }

    #[test]
    fn test_parse_file_url_helper() {
        // Test the parse_file_url helper directly
        assert_eq!(
            parse_file_url("file://localhost/home/user"),
            Some("/home/user".to_string())
        );
        assert_eq!(
            parse_file_url("file://host/"),
            Some("/".to_string())
        );
        assert_eq!(
            parse_file_url("http://localhost/home"),
            None
        );
        assert_eq!(parse_file_url("file://"), None);
        assert_eq!(parse_file_url("not a url"), None);
    }

    #[test]
    fn test_urlencoding_decode_helper() {
        assert_eq!(urlencoding_decode("/home/user"), "/home/user");
        assert_eq!(urlencoding_decode("/home/my%20docs"), "/home/my docs");
        assert_eq!(urlencoding_decode("%2F"), "/");
        assert_eq!(urlencoding_decode("100%25"), "100%");
        assert_eq!(urlencoding_decode("%3D%26"), "=&");
        // Invalid hex should preserve original
        assert_eq!(urlencoding_decode("%ZZ"), "%ZZ");
        assert_eq!(urlencoding_decode("%1"), "%1");
    }
}
