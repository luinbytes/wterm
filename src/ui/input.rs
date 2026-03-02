//! Keyboard input handling for the terminal emulator.
//!
//! Converts winit keyboard events into terminal escape sequences.
#![allow(dead_code)]

use winit::event::{ElementState, KeyEvent, Modifiers, WindowEvent};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

/// Represents a key sequence to be sent to the terminal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalInput {
    /// Regular character input
    Char(char),
    /// Escape sequence (e.g., arrow keys, function keys)
    Escape(String),
    /// No output (modifier only, etc.)
    None,
}

/// Tracks the state of keyboard modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl ModifierState {
    /// Creates a new modifier state with all modifiers released
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates modifier state from a winit Modifiers event
    pub fn update(&mut self, modifiers: Modifiers) {
        let state = modifiers.state();
        self.shift = state.shift_key();
        self.ctrl = state.control_key();
        self.alt = state.alt_key();
        self.super_key = state.super_key();
    }

    /// Updates modifier state from a ModifiersState directly
    pub fn update_from_state(&mut self, state: winit::keyboard::ModifiersState) {
        self.shift = state.shift_key();
        self.ctrl = state.control_key();
        self.alt = state.alt_key();
        self.super_key = state.super_key();
    }

    /// Returns the modifier number for escape sequences (1-8)
    /// Bit positions: Shift(1), Alt(2), Ctrl(4), Super(8) - but typically 1-8
    fn as_escape_modifier(&self) -> u8 {
        let mut modifier = 0u8;
        if self.shift {
            modifier |= 1;
        }
        if self.alt {
            modifier |= 2;
        }
        if self.ctrl {
            modifier |= 4;
        }
        // Super is sometimes mapped differently, but we include it for completeness
        if self.super_key {
            modifier |= 8;
        }
        modifier
    }

    /// Check if any modifier is pressed
    pub fn any_pressed(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.super_key
    }
}

/// Handles keyboard input conversion for terminal emulation
pub struct InputHandler {
    /// Current state of modifier keys
    modifiers: ModifierState,
}

impl InputHandler {
    /// Creates a new input handler
    pub fn new() -> Self {
        Self {
            modifiers: ModifierState::new(),
        }
    }

    /// Handle a window event and optionally return terminal input
    pub fn handle_event(&mut self, event: &WindowEvent) -> TerminalInput {
        match event {
            WindowEvent::KeyboardInput { event, .. } => self.handle_key_event(event),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers.update(*modifiers);
                TerminalInput::None
            }
            _ => TerminalInput::None,
        }
    }

    /// Process a keyboard event and return the corresponding terminal input
    pub fn handle_key_event(&mut self, key_event: &KeyEvent) -> TerminalInput {
        // Only process key press events, not releases
        if key_event.state != ElementState::Pressed {
            return TerminalInput::None;
        }

        let physical_key = key_event.physical_key;
        let logical_key = &key_event.logical_key;

        // Handle special keys based on physical key code
        if let PhysicalKey::Code(code) = physical_key {
            if let Some(escape) = self.handle_special_key(code) {
                return escape;
            }
        }

        // Handle based on logical key
        match logical_key {
            Key::Named(named) => self.handle_named_key(*named),
            Key::Character(ch) => self.handle_character(ch.as_str()),
            _ => TerminalInput::None,
        }
    }

    /// Handle special keys that produce escape sequences
    fn handle_special_key(&mut self, code: KeyCode) -> Option<TerminalInput> {
        let escape = match code {
            // Arrow keys
            KeyCode::ArrowUp => self.apply_modifiers("\x1b[A", "\x1b[1", 'A'),
            KeyCode::ArrowDown => self.apply_modifiers("\x1b[B", "\x1b[1", 'B'),
            KeyCode::ArrowRight => self.apply_modifiers("\x1b[C", "\x1b[1", 'C'),
            KeyCode::ArrowLeft => self.apply_modifiers("\x1b[D", "\x1b[1", 'D'),

            // Navigation keys
            KeyCode::Home => self.apply_modifiers("\x1b[H", "\x1b[1", 'H'),
            KeyCode::End => self.apply_modifiers("\x1b[F", "\x1b[1", 'F'),
            KeyCode::Insert => self.apply_modifiers("\x1b[2~", "\x1b[2", '~'),
            KeyCode::Delete => self.apply_modifiers("\x1b[3~", "\x1b[3", '~'),
            KeyCode::PageUp => self.apply_modifiers("\x1b[5~", "\x1b[5", '~'),
            KeyCode::PageDown => self.apply_modifiers("\x1b[6~", "\x1b[6", '~'),

            // Function keys F1-F12
            KeyCode::F1 => self.f_key(1),
            KeyCode::F2 => self.f_key(2),
            KeyCode::F3 => self.f_key(3),
            KeyCode::F4 => self.f_key(4),
            KeyCode::F5 => self.f_key(5),
            KeyCode::F6 => self.f_key(6),
            KeyCode::F7 => self.f_key(7),
            KeyCode::F8 => self.f_key(8),
            KeyCode::F9 => self.f_key(9),
            KeyCode::F10 => self.f_key(10),
            KeyCode::F11 => self.f_key(11),
            KeyCode::F12 => self.f_key(12),

            // Numpad keys
            KeyCode::NumpadEnter => TerminalInput::Char('\r'),
            KeyCode::NumpadDivide => TerminalInput::Char('/'),
            KeyCode::NumpadMultiply => TerminalInput::Char('*'),
            KeyCode::NumpadSubtract => TerminalInput::Char('-'),
            KeyCode::NumpadAdd => TerminalInput::Char('+'),
            KeyCode::NumpadDecimal => TerminalInput::Char('.'),

            // Other special keys
            KeyCode::Escape => TerminalInput::Char('\x1b'),
            KeyCode::Tab => {
                if self.modifiers.shift {
                    TerminalInput::Escape("\x1b[Z".to_string())
                } else {
                    TerminalInput::Char('\t')
                }
            }
            KeyCode::Backspace => {
                if self.modifiers.ctrl {
                    TerminalInput::Char('\x08') // Ctrl-Backspace
                } else {
                    TerminalInput::Char('\x7f') // DEL character for backspace
                }
            }
            KeyCode::Enter => TerminalInput::Char('\r'),

            _ => return None,
        };

        Some(escape)
    }

    /// Handle named keys (like Space, etc.)
    fn handle_named_key(&mut self, named: NamedKey) -> TerminalInput {
        match named {
            NamedKey::Space => {
                if self.modifiers.ctrl {
                    TerminalInput::Char('\x00')
                } else {
                    TerminalInput::Char(' ')
                }
            }
            NamedKey::Tab => {
                if self.modifiers.shift {
                    TerminalInput::Escape("\x1b[Z".to_string())
                } else {
                    TerminalInput::Char('\t')
                }
            }
            NamedKey::Enter => TerminalInput::Char('\r'),
            NamedKey::Backspace => TerminalInput::Char('\x7f'),
            NamedKey::Escape => TerminalInput::Char('\x1b'),
            _ => TerminalInput::None,
        }
    }

    /// Handle regular character input with modifiers
    fn handle_character(&mut self, ch: &str) -> TerminalInput {
        if ch.is_empty() {
            return TerminalInput::None;
        }

        let first_char = ch.chars().next().unwrap();

        // Handle Ctrl+letter combinations
        if self.modifiers.ctrl {
            if let Some(ctrl_char) = self.ctrl_char(first_char) {
                return TerminalInput::Char(ctrl_char);
            }
        }

        // Handle Alt prefix
        if self.modifiers.alt {
            // Alt+key sends ESC followed by the character
            return TerminalInput::Escape(format!("\x1b{}", ch));
        }

        // Regular character
        TerminalInput::Char(first_char)
    }

    /// Convert a character to its Ctrl equivalent
    fn ctrl_char(&self, c: char) -> Option<char> {
        let code = c.to_ascii_lowercase() as u32;
        if code >= 'a' as u32 && code <= 'z' as u32 {
            // Ctrl+A through Ctrl+Z map to 0x01 through 0x1A
            Some((code - 'a' as u32 + 1) as u8 as char)
        } else if code >= '0' as u32 && code <= '9' as u32 {
            // Some terminals map Ctrl+digit to specific codes
            match c {
                '2' => Some('\x00'), // Ctrl+2 -> NUL
                '3' => Some('\x1b'), // Ctrl+3 -> ESC
                '4' => Some('\x1c'), // Ctrl+4 -> FS
                '5' => Some('\x1d'), // Ctrl+5 -> GS
                '6' => Some('\x1e'), // Ctrl+6 -> RS
                '7' => Some('\x1f'), // Ctrl+7 -> US
                '8' => Some('\x7f'), // Ctrl+8 -> DEL
                _ => None,
            }
        } else {
            // Handle special cases
            match c {
                '[' => Some('\x1b'),  // Ctrl+[ -> ESC
                '\\' => Some('\x1c'), // Ctrl+\ -> FS
                ']' => Some('\x1d'),  // Ctrl+] -> GS
                '^' => Some('\x1e'),  // Ctrl+^ -> RS
                '_' => Some('\x1f'),  // Ctrl+_ -> US
                _ => None,
            }
        }
    }

    /// Apply modifier to escape sequence
    /// base: the unmodified escape sequence
    /// prefix: the CSI prefix for modified sequences (e.g., "\x1b[1")
    /// suffix: the final character (e.g., 'A' for up arrow)
    fn apply_modifiers(&self, base: &str, prefix: &str, suffix: char) -> TerminalInput {
        if !self.modifiers.any_pressed() {
            return TerminalInput::Escape(base.to_string());
        }

        let modifier = self.modifiers.as_escape_modifier();
        if modifier > 0 {
            TerminalInput::Escape(format!("{};{}{}", prefix, modifier + 1, suffix))
        } else {
            TerminalInput::Escape(base.to_string())
        }
    }

    /// Generate function key escape sequence with modifiers
    fn f_key(&self, num: u8) -> TerminalInput {
        let modifier = self.modifiers.as_escape_modifier();

        // F1-F4 use SS3 sequences, F5-F12 use CSI sequences
        let escape = match num {
            1 => {
                if modifier > 0 {
                    format!("\x1b[1;{}P", modifier + 1)
                } else {
                    "\x1bOP".to_string()
                }
            }
            2 => {
                if modifier > 0 {
                    format!("\x1b[1;{}Q", modifier + 1)
                } else {
                    "\x1bOQ".to_string()
                }
            }
            3 => {
                if modifier > 0 {
                    format!("\x1b[1;{}R", modifier + 1)
                } else {
                    "\x1bOR".to_string()
                }
            }
            4 => {
                if modifier > 0 {
                    format!("\x1b[1;{}S", modifier + 1)
                } else {
                    "\x1bOS".to_string()
                }
            }
            5 => {
                if modifier > 0 {
                    format!("\x1b[15;{}~", modifier + 1)
                } else {
                    "\x1b[15~".to_string()
                }
            }
            6 => {
                if modifier > 0 {
                    format!("\x1b[17;{}~", modifier + 1)
                } else {
                    "\x1b[17~".to_string()
                }
            }
            7 => {
                if modifier > 0 {
                    format!("\x1b[18;{}~", modifier + 1)
                } else {
                    "\x1b[18~".to_string()
                }
            }
            8 => {
                if modifier > 0 {
                    format!("\x1b[19;{}~", modifier + 1)
                } else {
                    "\x1b[19~".to_string()
                }
            }
            9 => {
                if modifier > 0 {
                    format!("\x1b[20;{}~", modifier + 1)
                } else {
                    "\x1b[20~".to_string()
                }
            }
            10 => {
                if modifier > 0 {
                    format!("\x1b[21;{}~", modifier + 1)
                } else {
                    "\x1b[21~".to_string()
                }
            }
            11 => {
                if modifier > 0 {
                    format!("\x1b[23;{}~", modifier + 1)
                } else {
                    "\x1b[23~".to_string()
                }
            }
            12 => {
                if modifier > 0 {
                    format!("\x1b[24;{}~", modifier + 1)
                } else {
                    "\x1b[24~".to_string()
                }
            }
            _ => return TerminalInput::None,
        };

        TerminalInput::Escape(escape)
    }

    /// Get a mutable reference to the modifier state
    pub fn modifiers_mut(&mut self) -> &mut ModifierState {
        &mut self.modifiers
    }

    /// Get the current modifier state
    pub fn modifiers(&self) -> &ModifierState {
        &self.modifiers
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalInput {
    /// Convert the terminal input to bytes for sending to the PTY
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TerminalInput::Char(c) => {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf);
                buf[..c.len_utf8()].to_vec()
            }
            TerminalInput::Escape(s) => s.as_bytes().to_vec(),
            TerminalInput::None => Vec::new(),
        }
    }

    /// Check if this input represents actual data to send
    pub fn has_output(&self) -> bool {
        !matches!(self, TerminalInput::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // TerminalInput Tests
    // ========================================

    #[test]
    fn test_terminal_input_to_bytes_char() {
        let char_input = TerminalInput::Char('a');
        assert_eq!(char_input.to_bytes(), vec![b'a']);
    }

    #[test]
    fn test_terminal_input_to_bytes_multibyte_char() {
        // Test UTF-8 encoding of multi-byte characters
        let char_input = TerminalInput::Char('€');
        assert_eq!(char_input.to_bytes(), vec![0xE2, 0x82, 0xAC]);
    }

    #[test]
    fn test_terminal_input_to_bytes_escape() {
        let escape_input = TerminalInput::Escape("\x1b[A".to_string());
        assert_eq!(escape_input.to_bytes(), vec![0x1b, b'[', b'A']);
    }

    #[test]
    fn test_terminal_input_to_bytes_none() {
        let none_input = TerminalInput::None;
        assert!(none_input.to_bytes().is_empty());
    }

    #[test]
    fn test_terminal_input_has_output() {
        assert!(TerminalInput::Char('a').has_output());
        assert!(TerminalInput::Escape("\x1b[A".to_string()).has_output());
        assert!(!TerminalInput::None.has_output());
    }

    #[test]
    fn test_terminal_input_debug_clone_partial_eq() {
        let input = TerminalInput::Char('x');
        let cloned = input.clone();
        assert_eq!(input, cloned);

        let escaped = TerminalInput::Escape("\x1b".to_string());
        assert_ne!(input, escaped);
    }

    // ========================================
    // ModifierState Tests
    // ========================================

    #[test]
    fn test_modifier_state_new() {
        let state = ModifierState::new();
        assert!(!state.shift);
        assert!(!state.ctrl);
        assert!(!state.alt);
        assert!(!state.super_key);
    }

    #[test]
    fn test_modifier_state_default() {
        let state = ModifierState::default();
        assert!(!state.any_pressed());
    }

    #[test]
    fn test_modifier_state_any_pressed() {
        let mut state = ModifierState::new();
        assert!(!state.any_pressed());

        state.shift = true;
        assert!(state.any_pressed());

        state.shift = false;
        state.ctrl = true;
        assert!(state.any_pressed());

        state.ctrl = false;
        state.alt = true;
        assert!(state.any_pressed());

        state.alt = false;
        state.super_key = true;
        assert!(state.any_pressed());
    }

    #[test]
    fn test_modifier_state_as_escape_modifier_shift() {
        let mut state = ModifierState::new();
        state.shift = true;
        assert_eq!(state.as_escape_modifier(), 1);
    }

    #[test]
    fn test_modifier_state_as_escape_modifier_alt() {
        let mut state = ModifierState::new();
        state.alt = true;
        assert_eq!(state.as_escape_modifier(), 2);
    }

    #[test]
    fn test_modifier_state_as_escape_modifier_ctrl() {
        let mut state = ModifierState::new();
        state.ctrl = true;
        assert_eq!(state.as_escape_modifier(), 4);
    }

    #[test]
    fn test_modifier_state_as_escape_modifier_super() {
        let mut state = ModifierState::new();
        state.super_key = true;
        assert_eq!(state.as_escape_modifier(), 8);
    }

    #[test]
    fn test_modifier_state_as_escape_modifier_combinations() {
        let mut state = ModifierState::new();

        // Shift + Alt = 3
        state.shift = true;
        state.alt = true;
        assert_eq!(state.as_escape_modifier(), 3);

        // Shift + Alt + Ctrl = 7
        state.ctrl = true;
        assert_eq!(state.as_escape_modifier(), 7);

        // All modifiers = 15
        state.super_key = true;
        assert_eq!(state.as_escape_modifier(), 15);
    }

    #[test]
    fn test_modifier_state_copy() {
        let mut state = ModifierState::new();
        state.shift = true;
        state.ctrl = true;

        let copied = state; // Copy trait
        assert_eq!(copied.shift, true);
        assert_eq!(copied.ctrl, true);
    }

    // ========================================
    // InputHandler Tests
    // ========================================

    #[test]
    fn test_input_handler_new() {
        let handler = InputHandler::new();
        assert!(!handler.modifiers().shift);
        assert!(!handler.modifiers().ctrl);
        assert!(!handler.modifiers().alt);
        assert!(!handler.modifiers().super_key);
    }

    #[test]
    fn test_input_handler_default() {
        let handler = InputHandler::default();
        assert!(!handler.modifiers().any_pressed());
    }

    #[test]
    fn test_input_handler_modifiers_accessors() {
        let mut handler = InputHandler::new();
        assert!(!handler.modifiers().shift);

        handler.modifiers_mut().shift = true;
        assert!(handler.modifiers().shift);
    }

    // ----------------------------------------
    // ctrl_char tests
    // ----------------------------------------

    #[test]
    fn test_ctrl_char_lowercase_letters() {
        let handler = InputHandler::new();

        // Ctrl+A through Ctrl+Z map to 0x01 through 0x1A
        assert_eq!(handler.ctrl_char('a'), Some('\x01'));
        assert_eq!(handler.ctrl_char('b'), Some('\x02'));
        assert_eq!(handler.ctrl_char('m'), Some('\x0d'));
        assert_eq!(handler.ctrl_char('z'), Some('\x1a'));
    }

    #[test]
    fn test_ctrl_char_uppercase_letters() {
        let handler = InputHandler::new();

        // Uppercase should be treated same as lowercase
        assert_eq!(handler.ctrl_char('A'), Some('\x01'));
        assert_eq!(handler.ctrl_char('Z'), Some('\x1a'));
    }

    #[test]
    fn test_ctrl_char_digits() {
        let handler = InputHandler::new();

        assert_eq!(handler.ctrl_char('0'), None);
        assert_eq!(handler.ctrl_char('1'), None);
        assert_eq!(handler.ctrl_char('2'), Some('\x00')); // NUL
        assert_eq!(handler.ctrl_char('3'), Some('\x1b')); // ESC
        assert_eq!(handler.ctrl_char('4'), Some('\x1c')); // FS
        assert_eq!(handler.ctrl_char('5'), Some('\x1d')); // GS
        assert_eq!(handler.ctrl_char('6'), Some('\x1e')); // RS
        assert_eq!(handler.ctrl_char('7'), Some('\x1f')); // US
        assert_eq!(handler.ctrl_char('8'), Some('\x7f')); // DEL
        assert_eq!(handler.ctrl_char('9'), None);
    }

    #[test]
    fn test_ctrl_char_special_characters() {
        let handler = InputHandler::new();

        assert_eq!(handler.ctrl_char('['), Some('\x1b'));  // ESC
        assert_eq!(handler.ctrl_char('\\'), Some('\x1c')); // FS
        assert_eq!(handler.ctrl_char(']'), Some('\x1d'));  // GS
        assert_eq!(handler.ctrl_char('^'), Some('\x1e'));  // RS
        assert_eq!(handler.ctrl_char('_'), Some('\x1f'));  // US
    }

    #[test]
    fn test_ctrl_char_unsupported() {
        let handler = InputHandler::new();

        // Characters outside the supported ranges
        assert_eq!(handler.ctrl_char('!'), None);
        assert_eq!(handler.ctrl_char('@'), None);
        assert_eq!(handler.ctrl_char(' '), None);
    }

    // ----------------------------------------
    // f_key tests
    // ----------------------------------------

    #[test]
    fn test_f_key_without_modifiers() {
        let handler = InputHandler::new();

        // F1-F4 use SS3 sequences
        assert_eq!(handler.f_key(1), TerminalInput::Escape("\x1bOP".to_string()));
        assert_eq!(handler.f_key(2), TerminalInput::Escape("\x1bOQ".to_string()));
        assert_eq!(handler.f_key(3), TerminalInput::Escape("\x1bOR".to_string()));
        assert_eq!(handler.f_key(4), TerminalInput::Escape("\x1bOS".to_string()));

        // F5-F12 use CSI sequences
        assert_eq!(handler.f_key(5), TerminalInput::Escape("\x1b[15~".to_string()));
        assert_eq!(handler.f_key(6), TerminalInput::Escape("\x1b[17~".to_string()));
        assert_eq!(handler.f_key(7), TerminalInput::Escape("\x1b[18~".to_string()));
        assert_eq!(handler.f_key(8), TerminalInput::Escape("\x1b[19~".to_string()));
        assert_eq!(handler.f_key(9), TerminalInput::Escape("\x1b[20~".to_string()));
        assert_eq!(handler.f_key(10), TerminalInput::Escape("\x1b[21~".to_string()));
        assert_eq!(handler.f_key(11), TerminalInput::Escape("\x1b[23~".to_string()));
        assert_eq!(handler.f_key(12), TerminalInput::Escape("\x1b[24~".to_string()));
    }

    #[test]
    fn test_f_key_with_shift_modifier() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().shift = true;

        // Shift modifier adds ";2" prefix
        assert_eq!(handler.f_key(1), TerminalInput::Escape("\x1b[1;2P".to_string()));
        assert_eq!(handler.f_key(5), TerminalInput::Escape("\x1b[15;2~".to_string()));
    }

    #[test]
    fn test_f_key_with_alt_modifier() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().alt = true;

        // Alt modifier = 2, so modifier+1 = 3
        assert_eq!(handler.f_key(1), TerminalInput::Escape("\x1b[1;3P".to_string()));
        assert_eq!(handler.f_key(12), TerminalInput::Escape("\x1b[24;3~".to_string()));
    }

    #[test]
    fn test_f_key_with_ctrl_modifier() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().ctrl = true;

        // Ctrl modifier = 4, so modifier+1 = 5
        assert_eq!(handler.f_key(1), TerminalInput::Escape("\x1b[1;5P".to_string()));
        assert_eq!(handler.f_key(5), TerminalInput::Escape("\x1b[15;5~".to_string()));
    }

    #[test]
    fn test_f_key_invalid() {
        let handler = InputHandler::new();

        assert_eq!(handler.f_key(0), TerminalInput::None);
        assert_eq!(handler.f_key(13), TerminalInput::None);
        assert_eq!(handler.f_key(255), TerminalInput::None);
    }

    // ----------------------------------------
    // apply_modifiers tests
    // ----------------------------------------

    #[test]
    fn test_apply_modifiers_no_modifiers() {
        let handler = InputHandler::new();

        let result = handler.apply_modifiers("\x1b[A", "\x1b[1", 'A');
        assert_eq!(result, TerminalInput::Escape("\x1b[A".to_string()));
    }

    #[test]
    fn test_apply_modifiers_with_shift() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().shift = true;

        let result = handler.apply_modifiers("\x1b[A", "\x1b[1", 'A');
        assert_eq!(result, TerminalInput::Escape("\x1b[1;2A".to_string()));
    }

    #[test]
    fn test_apply_modifiers_with_ctrl_alt() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().ctrl = true;
        handler.modifiers_mut().alt = true;

        // Ctrl(4) + Alt(2) = 6, so modifier+1 = 7
        let result = handler.apply_modifiers("\x1b[B", "\x1b[1", 'B');
        assert_eq!(result, TerminalInput::Escape("\x1b[1;7B".to_string()));
    }

    // ----------------------------------------
    // handle_character tests
    // ----------------------------------------

    #[test]
    fn test_handle_character_regular() {
        let mut handler = InputHandler::new();

        assert_eq!(handler.handle_character("a"), TerminalInput::Char('a'));
        assert_eq!(handler.handle_character("Z"), TerminalInput::Char('Z'));
        assert_eq!(handler.handle_character("1"), TerminalInput::Char('1'));
    }

    #[test]
    fn test_handle_character_empty() {
        let mut handler = InputHandler::new();

        assert_eq!(handler.handle_character(""), TerminalInput::None);
    }

    #[test]
    fn test_handle_character_with_ctrl() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().ctrl = true;

        assert_eq!(handler.handle_character("a"), TerminalInput::Char('\x01'));
        assert_eq!(handler.handle_character("m"), TerminalInput::Char('\x0d'));
    }

    #[test]
    fn test_handle_character_with_alt() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().alt = true;

        // Alt+key sends ESC followed by the character
        assert_eq!(
            handler.handle_character("a"),
            TerminalInput::Escape("\x1ba".to_string())
        );
        assert_eq!(
            handler.handle_character("x"),
            TerminalInput::Escape("\x1bx".to_string())
        );
    }

    #[test]
    fn test_handle_character_ctrl_takes_precedence_over_alt() {
        let mut handler = InputHandler::new();
        handler.modifiers_mut().ctrl = true;
        handler.modifiers_mut().alt = true;

        // Ctrl should be processed first
        assert_eq!(handler.handle_character("a"), TerminalInput::Char('\x01'));
    }

    // ========================================
    // Integration-style tests for modifiers
    // ========================================

    #[test]
    fn test_modifier_state_update_from_state_all_modifiers() {
        use winit::keyboard::ModifiersState;

        let mut state = ModifierState::new();
        let mods = ModifiersState::all();
        state.update_from_state(mods);

        assert!(state.shift);
        assert!(state.ctrl);
        assert!(state.alt);
        assert!(state.super_key);
    }
}
