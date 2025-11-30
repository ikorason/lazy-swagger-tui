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
    /// The content stored as lines for easier multi-line editing
    lines: Vec<String>,

    /// Cursor row (line number, 0-indexed)
    cursor_row: usize,

    /// Cursor column (character position in current line, 0-indexed)
    cursor_col: usize,

    /// Vertical scroll offset (which line is at top of viewport)
    offset_y: usize,

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
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            offset_y: 0,
            dirty: false,
            content_type: ContentType::Json,
        }
    }

    /// Create a new editor with initial content
    #[allow(dead_code)] // Reserved for future use
    pub fn with_content(content: String) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };

        let cursor_row = lines.len().saturating_sub(1);
        let cursor_col = lines.last().map(|l| l.len()).unwrap_or(0);

        Self {
            lines,
            cursor_row,
            cursor_col,
            offset_y: 0,
            dirty: false,
            content_type: ContentType::Json,
        }
    }

    /// Get the current content as a single string
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Get the current cursor position as (row, col)
    #[allow(dead_code)] // Reserved for future cursor rendering
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Get the lines for rendering
    #[allow(dead_code)] // Reserved for future rendering
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get content with cursor marker (█) for rendering
    pub fn content_with_cursor(&self) -> String {
        let mut result = Vec::new();

        for (row_idx, line) in self.lines.iter().enumerate() {
            if row_idx == self.cursor_row {
                // This is the cursor line - insert cursor marker
                let cursor_col = self.cursor_col.min(line.len());
                let before = &line[..cursor_col];
                let after = &line[cursor_col..];
                result.push(format!("{before}█{after}"));
            } else {
                result.push(line.clone());
            }
        }

        result.join("\n")
    }

    /// Get cursor position string for display (e.g., "Line 5, Col 12")
    pub fn cursor_position_display(&self) -> String {
        format!("Ln {}, Col {}", self.cursor_row + 1, self.cursor_col + 1)
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
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.offset_y = 0;
        self.dirty = true;
    }

    /// Set content (replaces all existing content)
    pub fn set_content(&mut self, content: String) {
        self.lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|s| s.to_string()).collect()
        };

        // Place cursor at end of last line
        self.cursor_row = self.lines.len().saturating_sub(1);
        self.cursor_col = self.lines.last().map(|l| l.len()).unwrap_or(0);
        self.offset_y = 0;
        self.dirty = true;
    }

    /// Insert a character at the current cursor position
    pub fn insert_char(&mut self, c: char) {
        // Ensure we have a valid cursor position
        if self.cursor_row >= self.lines.len() {
            self.cursor_row = self.lines.len().saturating_sub(1);
        }

        let line = &mut self.lines[self.cursor_row];
        let cursor_col = self.cursor_col.min(line.len());

        line.insert(cursor_col, c);
        self.cursor_col = cursor_col + 1;
        self.dirty = true;
    }

    /// Insert a string at the current cursor position
    /// Handles multi-line strings (with \n) correctly
    pub fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        // Ensure we have a valid cursor position
        if self.cursor_row >= self.lines.len() {
            self.cursor_row = self.lines.len().saturating_sub(1);
        }

        let lines_to_insert: Vec<&str> = s.split('\n').collect();

        if lines_to_insert.len() == 1 {
            // Single line insert - simple case
            let line = &mut self.lines[self.cursor_row];
            let cursor_col = self.cursor_col.min(line.len());
            line.insert_str(cursor_col, s);
            self.cursor_col = cursor_col + s.len();
        } else {
            // Multi-line insert - need to split current line
            let current_line = self.lines[self.cursor_row].clone();
            let cursor_col = self.cursor_col.min(current_line.len());

            let before_cursor = &current_line[..cursor_col];
            let after_cursor = &current_line[cursor_col..];

            // First line: before cursor + first inserted line
            self.lines[self.cursor_row] = format!("{}{}", before_cursor, lines_to_insert[0]);

            // Middle lines: inserted as-is
            for (i, &line) in lines_to_insert
                .iter()
                .enumerate()
                .skip(1)
                .take(lines_to_insert.len() - 2)
            {
                self.lines.insert(self.cursor_row + i, line.to_string());
            }

            // Last line: last inserted line + after cursor
            let last_inserted = lines_to_insert.last().unwrap();
            let last_line_idx = self.cursor_row + lines_to_insert.len() - 1;
            self.lines
                .insert(last_line_idx, format!("{last_inserted}{after_cursor}"));

            // Update cursor to end of inserted content
            self.cursor_row = last_line_idx;
            self.cursor_col = last_inserted.len();
        }

        self.dirty = true;
    }

    /// Insert a string with smart quote normalization (useful for JSON)
    /// Converts curly quotes to straight quotes for JSON compatibility
    pub fn insert_str_normalized(&mut self, s: &str) {
        let normalized = s
            .replace(['\u{201C}', '\u{201D}'], "\"") // Right double quote "
            .replace(['\u{2018}', '\u{2019}'], "'"); // Right single quote '

        self.insert_str(&normalized);
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_char_before_cursor(&mut self) -> bool {
        if self.cursor_col > 0 {
            // Delete character on current line
            let line = &mut self.lines[self.cursor_row];
            line.remove(self.cursor_col - 1);
            self.cursor_col -= 1;
            self.dirty = true;
            true
        } else if self.cursor_row > 0 {
            // At start of line - join with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.dirty = true;
            true
        } else {
            // At start of document
            false
        }
    }

    /// Delete the character after the cursor (delete key)
    pub fn delete_char_after_cursor(&mut self) -> bool {
        let line = &self.lines[self.cursor_row];

        if self.cursor_col < line.len() {
            // Delete character on current line
            self.lines[self.cursor_row].remove(self.cursor_col);
            self.dirty = true;
            true
        } else if self.cursor_row < self.lines.len() - 1 {
            // At end of line - join with next line
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
            self.dirty = true;
            true
        } else {
            // At end of document
            false
        }
    }

    /// Move cursor to the left by one character
    pub fn move_cursor_left(&mut self) -> bool {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            true
        } else if self.cursor_row > 0 {
            // Move to end of previous line
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            true
        } else {
            false
        }
    }

    /// Move cursor to the right by one character
    pub fn move_cursor_right(&mut self) -> bool {
        let line = &self.lines[self.cursor_row];

        if self.cursor_col < line.len() {
            self.cursor_col += 1;
            true
        } else if self.cursor_row < self.lines.len() - 1 {
            // Move to start of next line
            self.cursor_row += 1;
            self.cursor_col = 0;
            true
        } else {
            false
        }
    }

    /// Move cursor to start of current line
    pub fn move_cursor_to_start(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of current line
    pub fn move_cursor_to_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    /// Format content as JSON (prettify)
    /// Returns Ok(()) if formatting succeeded, Err with the parse error if invalid JSON
    pub fn format_json(&mut self) -> Result<(), String> {
        let content = self.content();
        match serde_json::from_str::<Value>(&content) {
            Ok(json) => {
                // Valid JSON - prettify it
                let formatted = serde_json::to_string_pretty(&json).unwrap_or(content);
                self.set_content(formatted);
                Ok(())
            }
            Err(e) => Err(format!("Invalid JSON: {e}")),
        }
    }

    /// Validate that content is valid JSON
    #[allow(dead_code)] // Reserved for future validation UI
    pub fn validate_json(&self) -> Result<(), String> {
        let content = self.content();
        serde_json::from_str::<Value>(&content)
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
            KeyCode::Up => self.move_cursor_up(),
            KeyCode::Down => self.move_cursor_down(),
            KeyCode::Enter => {
                self.insert_newline();
                true
            }
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
        while let Ok(true) = crossterm::event::poll(std::time::Duration::from_millis(0)) {
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

        let count = chars.len();
        let batch_str: String = chars.into_iter().collect();

        // Use normalized insertion for paste to handle smart quotes
        self.insert_str_normalized(&batch_str);
        count
    }

    /// Move cursor up by one line
    pub fn move_cursor_up(&mut self) -> bool {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            // Clamp column to new line length
            let line_len = self.lines[self.cursor_row].len();
            self.cursor_col = self.cursor_col.min(line_len);
            true
        } else {
            false
        }
    }

    /// Move cursor down by one line
    pub fn move_cursor_down(&mut self) -> bool {
        if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            // Clamp column to new line length
            let line_len = self.lines[self.cursor_row].len();
            self.cursor_col = self.cursor_col.min(line_len);
            true
        } else {
            false
        }
    }

    /// Insert a newline at the current cursor position
    pub fn insert_newline(&mut self) {
        let current_line = self.lines[self.cursor_row].clone();
        let cursor_col = self.cursor_col.min(current_line.len());

        // Split the current line at cursor
        let before_cursor = current_line[..cursor_col].to_string();
        let after_cursor = current_line[cursor_col..].to_string();

        // Update current line to be text before cursor
        self.lines[self.cursor_row] = before_cursor;

        // Insert new line with text after cursor
        self.lines.insert(self.cursor_row + 1, after_cursor);

        // Move cursor to start of new line
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.dirty = true;
    }

    // Future extension points (currently unimplemented):
    //
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
        assert_eq!(editor.cursor(), (0, 0));
        assert!(!editor.is_dirty());
    }

    #[test]
    fn test_with_content() {
        let editor = BodyEditor::with_content("hello".to_string());
        assert_eq!(editor.content(), "hello");
        assert_eq!(editor.cursor(), (0, 5)); // row 0, col 5
        assert!(!editor.is_dirty());
    }

    #[test]
    fn test_insert_char() {
        let mut editor = BodyEditor::new();
        editor.insert_char('a');
        assert_eq!(editor.content(), "a");
        assert_eq!(editor.cursor(), (0, 1));
        assert!(editor.is_dirty());
    }

    #[test]
    fn test_insert_str() {
        let mut editor = BodyEditor::new();
        editor.insert_str("hello");
        assert_eq!(editor.content(), "hello");
        assert_eq!(editor.cursor(), (0, 5));
        assert!(editor.is_dirty());
    }

    #[test]
    fn test_delete_char_before_cursor() {
        let mut editor = BodyEditor::with_content("hello".to_string());
        assert!(editor.delete_char_before_cursor());
        assert_eq!(editor.content(), "hell");
        assert_eq!(editor.cursor(), (0, 4));
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
        assert_eq!(editor.cursor(), (0, 4));
        assert!(editor.move_cursor_right());
        assert_eq!(editor.cursor(), (0, 5));
        assert!(!editor.move_cursor_right()); // At end
    }

    #[test]
    fn test_clear() {
        let mut editor = BodyEditor::with_content("hello".to_string());
        editor.clear();
        assert_eq!(editor.content(), "");
        assert_eq!(editor.cursor(), (0, 0));
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
    fn test_paste_and_format_flow() {
        // Simulate the exact flow that happens when user pastes JSON
        let mut editor = BodyEditor::new();

        // Simulate pasting compact JSON (what insert_str_normalized does)
        let compact_json = r#"{"name":"test","age":30,"items":[1,2,3]}"#;
        editor.insert_str_normalized(compact_json);

        // Verify it was inserted
        assert_eq!(editor.content(), compact_json);

        // Now format it (what happens after paste is detected)
        let result = editor.format_json();
        assert!(result.is_ok());

        // Verify it's now formatted with multiple lines
        let formatted = editor.content();
        println!("Formatted content:\n{formatted}");
        println!("Line count: {}", formatted.lines().count());
        assert!(formatted.contains("  ")); // Has indentation
        assert!(formatted.contains("\n")); // Has newlines

        // Verify content_with_cursor() shows the formatted version
        let with_cursor = editor.content_with_cursor();
        println!("\nWith cursor:\n{with_cursor}");
        println!("Line count: {}", with_cursor.lines().count());
        assert!(with_cursor.contains("  ")); // Has indentation
        assert!(with_cursor.contains("\n")); // Has newlines
        assert!(with_cursor.contains("█")); // Has cursor marker
    }

    #[test]
    fn test_insert_newline_splits_line() {
        let mut editor = BodyEditor::new();
        editor.insert_str("hello world");

        // Cursor should be at (0, 11) - end of "hello world"
        assert_eq!(editor.cursor(), (0, 11));

        // Move cursor to position 5 (after "hello")
        editor.cursor_col = 5;

        // Insert newline - should split the line
        editor.insert_newline();

        // Should now have 2 lines: "hello" and " world"
        assert_eq!(editor.lines().len(), 2);
        assert_eq!(editor.lines()[0], "hello");
        assert_eq!(editor.lines()[1], " world");

        // Cursor should be on line 2, column 0
        assert_eq!(editor.cursor(), (1, 0));
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
        assert_eq!(editor.cursor(), (0, 1)); // 1 character (not bytes)
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

    #[test]
    fn test_move_cursor_up_down() {
        let mut editor = BodyEditor::with_content("line1\nline2\nline3".to_string());

        // Start at end of last line
        assert_eq!(editor.cursor(), (2, 5));

        // Move up
        assert!(editor.move_cursor_up());
        assert_eq!(editor.cursor(), (1, 5));

        assert!(editor.move_cursor_up());
        assert_eq!(editor.cursor(), (0, 5));

        // Can't move up from first line
        assert!(!editor.move_cursor_up());
        assert_eq!(editor.cursor(), (0, 5));

        // Move down
        assert!(editor.move_cursor_down());
        assert_eq!(editor.cursor(), (1, 5));

        assert!(editor.move_cursor_down());
        assert_eq!(editor.cursor(), (2, 5));

        // Can't move down from last line
        assert!(!editor.move_cursor_down());
    }

    #[test]
    fn test_insert_newline() {
        let mut editor = BodyEditor::with_content("hello world".to_string());

        // Move cursor to after "hello"
        editor.cursor_col = 5;
        editor.insert_newline();

        assert_eq!(editor.content(), "hello\n world");
        assert_eq!(editor.cursor(), (1, 0));
    }

    #[test]
    fn test_multiline_paste() {
        let mut editor = BodyEditor::new();
        editor.insert_str("{\n  \"name\": \"test\"\n}");

        assert_eq!(editor.lines().len(), 3);
        assert_eq!(editor.lines()[0], "{");
        assert_eq!(editor.lines()[1], "  \"name\": \"test\"");
        assert_eq!(editor.lines()[2], "}");
        assert_eq!(editor.cursor(), (2, 1)); // After the closing brace
    }

    #[test]
    fn test_cursor_position_display() {
        let mut editor = BodyEditor::new();
        assert_eq!(editor.cursor_position_display(), "Ln 1, Col 1");

        editor.insert_str("hello\nworld");
        assert_eq!(editor.cursor_position_display(), "Ln 2, Col 6");
    }

    #[test]
    fn test_content_with_cursor() {
        let mut editor = BodyEditor::with_content("hello\nworld".to_string());

        // Cursor at end of last line
        assert_eq!(editor.cursor(), (1, 5));
        let content = editor.content_with_cursor();
        assert_eq!(content, "hello\nworld█");

        // Move cursor to middle of first line
        editor.cursor_row = 0;
        editor.cursor_col = 2;
        let content = editor.content_with_cursor();
        assert_eq!(content, "he█llo\nworld");
    }
}
