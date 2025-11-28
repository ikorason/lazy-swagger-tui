//! Body editor module
//!
//! This module provides a text editor specifically designed for editing request bodies
//! (primarily JSON). It encapsulates all editing state and operations, making it easy
//! to extend with features like:
//! - Cursor movement (left/right/up/down)
//! - Multi-line editing with proper line navigation
//! - Syntax highlighting for JSON
//! - Auto-indentation
//! - Bracket matching
//! - Undo/redo functionality

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;

/// A text editor for editing request bodies (primarily JSON)
#[derive(Debug, Clone)]
pub struct BodyEditor {
    /// The content being edited
    content: String,

    /// Cursor position (byte offset in content)
    cursor: usize,

    /// Whether the content has been modified since last save
    dirty: bool,

    /// Optional: Track content type for syntax-aware features
    content_type: ContentType,
}

/// Content type for the editor (enables syntax-specific features)
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum ContentType {
    Json,
    Xml,
    PlainText,
}

impl Default for BodyEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl BodyEditor {
    /// Create a new empty editor
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            dirty: false,
            content_type: ContentType::Json,
        }
    }

    /// Create a new editor with initial content
    #[allow(dead_code)] // Reserved for future use
    pub fn with_content(content: String) -> Self {
        let cursor = content.len(); // Place cursor at end
        Self {
            content,
            cursor,
            dirty: false,
            content_type: ContentType::Json,
        }
    }

    /// Get the current content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the current cursor position
    #[allow(dead_code)] // Reserved for future cursor rendering
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Check if content has been modified
    #[allow(dead_code)] // Reserved for future unsaved changes warning
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Set content type (for future syntax highlighting)
    #[allow(dead_code)] // Reserved for future syntax highlighting
    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.content_type = content_type;
    }

    /// Get content type
    #[allow(dead_code)] // Reserved for future syntax highlighting
    pub fn content_type(&self) -> &ContentType {
        &self.content_type
    }

    /// Clear all content and reset state
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
        self.dirty = true;
    }

    /// Set content (replaces all existing content)
    pub fn set_content(&mut self, content: String) {
        self.cursor = content.len();
        self.content = content;
        self.dirty = true;
    }

    /// Insert a character at the current cursor position
    pub fn insert_char(&mut self, c: char) {
        // Ensure cursor is at a valid UTF-8 boundary
        let cursor = self.clamp_cursor_to_boundary(self.cursor);
        self.content.insert(cursor, c);
        self.cursor = cursor + c.len_utf8();
        self.dirty = true;
    }

    /// Insert a string at the current cursor position
    pub fn insert_str(&mut self, s: &str) {
        let cursor = self.clamp_cursor_to_boundary(self.cursor);
        self.content.insert_str(cursor, s);
        self.cursor = cursor + s.len();
        self.dirty = true;
    }

    /// Insert a string with smart quote normalization (useful for JSON)
    /// Converts curly quotes to straight quotes for JSON compatibility
    pub fn insert_str_normalized(&mut self, s: &str) {
        let normalized = s
            .replace('\u{201C}', "\"") // Left double quote "
            .replace('\u{201D}', "\"") // Right double quote "
            .replace('\u{2018}', "'")  // Left single quote '
            .replace('\u{2019}', "'"); // Right single quote '

        let cursor = self.clamp_cursor_to_boundary(self.cursor);
        self.content.insert_str(cursor, &normalized);
        self.cursor = cursor + normalized.len();
        self.dirty = true;
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_char_before_cursor(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        // Find the previous character boundary
        let mut cursor = self.cursor;
        while cursor > 0 && !self.content.is_char_boundary(cursor - 1) {
            cursor -= 1;
        }
        if cursor > 0 {
            cursor -= 1;
        }

        self.content.remove(cursor);
        self.cursor = cursor;
        self.dirty = true;
        true
    }

    /// Delete the character after the cursor (delete key)
    pub fn delete_char_after_cursor(&mut self) -> bool {
        if self.cursor >= self.content.len() {
            return false;
        }

        let cursor = self.clamp_cursor_to_boundary(self.cursor);
        self.content.remove(cursor);
        self.dirty = true;
        true
    }

    /// Move cursor to the left by one character
    pub fn move_cursor_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        // Move to previous char boundary
        let mut new_cursor = self.cursor - 1;
        while new_cursor > 0 && !self.content.is_char_boundary(new_cursor) {
            new_cursor -= 1;
        }

        self.cursor = new_cursor;
        true
    }

    /// Move cursor to the right by one character
    pub fn move_cursor_right(&mut self) -> bool {
        if self.cursor >= self.content.len() {
            return false;
        }

        // Move to next char boundary
        let mut new_cursor = self.cursor + 1;
        while new_cursor < self.content.len() && !self.content.is_char_boundary(new_cursor) {
            new_cursor += 1;
        }

        self.cursor = new_cursor.min(self.content.len());
        true
    }

    /// Move cursor to start of content
    pub fn move_cursor_to_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end of content
    pub fn move_cursor_to_end(&mut self) {
        self.cursor = self.content.len();
    }

    /// Format content as JSON (prettify)
    /// Returns Ok(()) if formatting succeeded, Err with the parse error if invalid JSON
    pub fn format_json(&mut self) -> Result<(), String> {
        match serde_json::from_str::<Value>(&self.content) {
            Ok(json) => {
                // Valid JSON - prettify it
                self.content =
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| self.content.clone());
                self.cursor = self.content.len();
                self.dirty = true;
                Ok(())
            }
            Err(e) => Err(format!("Invalid JSON: {e}")),
        }
    }

    /// Validate that content is valid JSON
    #[allow(dead_code)] // Reserved for future validation UI
    pub fn validate_json(&self) -> Result<(), String> {
        serde_json::from_str::<Value>(&self.content)
            .map(|_| ())
            .map_err(|e| format!("Invalid JSON: {e}"))
    }

    /// Mark content as saved (clears dirty flag)
    #[allow(dead_code)] // Reserved for future dirty state tracking
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Handle a key event - returns true if the event was handled
    ///
    /// This provides a convenient way to handle common editing operations.
    /// For more fine-grained control, use the individual methods directly.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Backspace => self.delete_char_before_cursor(),
            KeyCode::Delete => self.delete_char_after_cursor(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Home => {
                self.move_cursor_to_start();
                true
            }
            KeyCode::End => {
                self.move_cursor_to_end();
                true
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_to_start();
                true
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_cursor_to_end();
                true
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.clear();
                true
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.insert_char(c);
                true
            }
            _ => false,
        }
    }

    /// Handle paste batching - collects multiple character events in quick succession
    ///
    /// This is useful for terminal paste operations where characters arrive rapidly.
    /// Automatically normalizes smart quotes to regular quotes for JSON compatibility.
    /// Returns the number of characters inserted.
    pub fn handle_paste_batch(&mut self, initial_char: char) -> usize {
        let mut chars = vec![initial_char];

        // Drain any immediately available character events
        loop {
            match crossterm::event::poll(std::time::Duration::from_millis(0)) {
                Ok(true) => {
                    if let Ok(Event::Key(next_key)) = crossterm::event::read() {
                        match next_key.code {
                            KeyCode::Char(next_c)
                                if !next_key.modifiers.contains(KeyModifiers::CONTROL) =>
                            {
                                chars.push(next_c);
                            }
                            _ => {
                                // Non-character or control key, stop batching
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        let count = chars.len();
        let batch_str: String = chars.into_iter().collect();

        // Use normalized insertion for paste to handle smart quotes
        self.insert_str_normalized(&batch_str);
        count
    }

    /// Clamp cursor to valid UTF-8 character boundary
    fn clamp_cursor_to_boundary(&self, cursor: usize) -> usize {
        let mut pos = cursor.min(self.content.len());
        while pos > 0 && !self.content.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    // Future extension points (currently unimplemented):
    //
    // pub fn move_cursor_up(&mut self) -> bool { ... }
    // pub fn move_cursor_down(&mut self) -> bool { ... }
    // pub fn get_cursor_line_col(&self) -> (usize, usize) { ... }
    // pub fn get_lines(&self) -> Vec<&str> { ... }
    // pub fn insert_newline(&mut self) { ... }
    // pub fn auto_indent(&mut self) { ... }
    // pub fn highlight_syntax(&self) -> Vec<(Range<usize>, Style)> { ... }
    // pub fn undo(&mut self) { ... }
    // pub fn redo(&mut self) { ... }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor() {
        let editor = BodyEditor::new();
        assert_eq!(editor.content(), "");
        assert_eq!(editor.cursor(), 0);
        assert!(!editor.is_dirty());
    }

    #[test]
    fn test_with_content() {
        let editor = BodyEditor::with_content("hello".to_string());
        assert_eq!(editor.content(), "hello");
        assert_eq!(editor.cursor(), 5);
        assert!(!editor.is_dirty());
    }

    #[test]
    fn test_insert_char() {
        let mut editor = BodyEditor::new();
        editor.insert_char('a');
        assert_eq!(editor.content(), "a");
        assert_eq!(editor.cursor(), 1);
        assert!(editor.is_dirty());
    }

    #[test]
    fn test_insert_str() {
        let mut editor = BodyEditor::new();
        editor.insert_str("hello");
        assert_eq!(editor.content(), "hello");
        assert_eq!(editor.cursor(), 5);
        assert!(editor.is_dirty());
    }

    #[test]
    fn test_delete_char_before_cursor() {
        let mut editor = BodyEditor::with_content("hello".to_string());
        assert!(editor.delete_char_before_cursor());
        assert_eq!(editor.content(), "hell");
        assert_eq!(editor.cursor(), 4);
    }

    #[test]
    fn test_delete_at_start() {
        let mut editor = BodyEditor::with_content("hello".to_string());
        editor.move_cursor_to_start();
        assert!(!editor.delete_char_before_cursor());
        assert_eq!(editor.content(), "hello");
    }

    #[test]
    fn test_move_cursor_left_right() {
        let mut editor = BodyEditor::with_content("hello".to_string());
        assert!(editor.move_cursor_left());
        assert_eq!(editor.cursor(), 4);
        assert!(editor.move_cursor_right());
        assert_eq!(editor.cursor(), 5);
        assert!(!editor.move_cursor_right()); // At end
    }

    #[test]
    fn test_clear() {
        let mut editor = BodyEditor::with_content("hello".to_string());
        editor.clear();
        assert_eq!(editor.content(), "");
        assert_eq!(editor.cursor(), 0);
        assert!(editor.is_dirty());
    }

    #[test]
    fn test_format_json_valid() {
        let mut editor = BodyEditor::with_content(r#"{"name":"test","age":30}"#.to_string());
        assert!(editor.format_json().is_ok());
        assert!(editor.content().contains("  ")); // Should be indented
        assert!(editor.content().contains("\"name\""));
    }

    #[test]
    fn test_format_json_invalid() {
        let mut editor = BodyEditor::with_content("{invalid json".to_string());
        assert!(editor.format_json().is_err());
        assert_eq!(editor.content(), "{invalid json"); // Content unchanged
    }

    #[test]
    fn test_validate_json() {
        let mut editor = BodyEditor::with_content(r#"{"valid": true}"#.to_string());
        assert!(editor.validate_json().is_ok());

        editor.set_content("{invalid}".to_string());
        assert!(editor.validate_json().is_err());
    }

    #[test]
    fn test_mark_saved() {
        let mut editor = BodyEditor::new();
        editor.insert_char('a');
        assert!(editor.is_dirty());
        editor.mark_saved();
        assert!(!editor.is_dirty());
    }

    #[test]
    fn test_utf8_handling() {
        let mut editor = BodyEditor::new();
        editor.insert_char('😀'); // Multi-byte emoji
        assert_eq!(editor.content(), "😀");
        assert_eq!(editor.cursor(), 4); // 4 bytes for this emoji
        assert!(editor.delete_char_before_cursor());
        assert_eq!(editor.content(), "");
    }

    #[test]
    fn test_smart_quote_normalization() {
        let mut editor = BodyEditor::new();

        // Test smart double quotes (curly quotes) - using Unicode escape sequences
        // \u{201C} = " (left double quote), \u{201D} = " (right double quote)
        let smart_quoted = "{\u{201C}username\u{201D}:\u{201D}test\u{201D}}";
        editor.insert_str_normalized(smart_quoted);
        assert_eq!(editor.content(), r#"{"username":"test"}"#);

        // Verify it formats as valid JSON
        assert!(editor.format_json().is_ok());
    }

    #[test]
    fn test_regular_quotes_unchanged() {
        let mut editor = BodyEditor::new();

        // Regular quotes should remain unchanged
        editor.insert_str_normalized(r#"{"username":"test"}"#);
        assert_eq!(editor.content(), r#"{"username":"test"}"#);
        assert!(editor.format_json().is_ok());
    }

    #[test]
    fn test_single_quote_normalization() {
        let mut editor = BodyEditor::new();

        // Smart single quotes - \u{2018} = ' (left), \u{2019} = ' (right)
        let smart_single = "{\u{2018}key\u{2019}:\u{2018}value\u{2019}}";
        editor.insert_str_normalized(smart_single);
        assert_eq!(editor.content(), "{'key':'value'}");
    }
}
