use std::io::{Read, Write};

use anyhow::Result;
use termion::event::Key;
use termion::input::TermRead;
use unicode_width::UnicodeWidthStr;

const USAGE_EDITOR: &str = "(post: ctrl-p / exit: ctrl-c)";
/// Terminal row where the message starts, below the channel and usage lines
const MESSAGE_ROW: u16 = 3;

/// Result of message editing
pub enum EditorResult {
    /// User submitted the message (Ctrl-P)
    Submitted(String),
    /// User cancelled the editor (Ctrl-C)
    Cancelled,
}

pub struct Editor {
    buffer: Vec<String>,
    cursor_line: usize,
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            buffer: vec![String::new()],
            cursor_line: 0,
        }
    }

    pub fn message(&self) -> String {
        self.buffer.join("\n")
    }

    /// Check if message input is needed
    pub fn needs_input(message: &str) -> bool {
        message.trim().is_empty()
    }

    /// Run the interactive message editor
    /// Returns EditorResult::Submitted(message) if submitted, EditorResult::Cancelled if cancelled
    pub fn run<R: Read, W: Write>(
        &mut self,
        stdin: R,
        stdout: &mut W,
        channel: &str,
    ) -> Result<EditorResult> {
        self.draw_header(stdout, channel);

        for c in stdin.keys() {
            match c? {
                Key::Ctrl('c') => return Ok(EditorResult::Cancelled),
                Key::Ctrl('p') => {
                    let msg = self.message();
                    if msg.trim().is_empty() {
                        self.clear();
                    } else {
                        return Ok(EditorResult::Submitted(msg));
                    }
                }
                Key::Char('\n') => {
                    self.new_line();
                }
                Key::Char(c) => {
                    self.insert(c);
                }
                Key::Up => {
                    self.cursor_up();
                }
                Key::Down => {
                    self.cursor_down();
                }
                Key::Backspace => {
                    self.backspace();
                }
                _ => {}
            }

            self.draw_message(stdout);
        }

        Ok(EditorResult::Submitted(self.message()))
    }

    pub fn draw_header(&self, stdout: &mut dyn Write, channel: &str) {
        write!(
            stdout,
            "{}{}#{}{}{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All,
            &channel,
            termion::cursor::Goto(1, 2),
            USAGE_EDITOR,
            termion::cursor::Goto(1, MESSAGE_ROW)
        )
        .unwrap();
        stdout.flush().unwrap();
    }

    /// Redraw the whole message, clearing what was below it (e.g. a removed line),
    /// and put the cursor at the end of the current line
    pub fn draw_message(&self, stdout: &mut dyn Write) {
        // Raw mode needs "\r\n" to return to the first column
        write!(
            stdout,
            "{}{}{}",
            termion::cursor::Goto(1, MESSAGE_ROW),
            termion::clear::AfterCursor,
            self.buffer.join("\r\n")
        )
        .unwrap();
        // Columns are display widths, so wide characters such as Japanese take two
        let col = self.buffer[self.cursor_line].width() as u16 + 1;
        let row = self.cursor_line as u16 + MESSAGE_ROW;
        write!(stdout, "{}", termion::cursor::Goto(col, row)).unwrap();
        stdout.flush().unwrap();
    }

    pub fn insert(&mut self, c: char) {
        self.buffer[self.cursor_line].push(c);
    }

    pub fn backspace(&mut self) {
        if !self.buffer[self.cursor_line].is_empty() {
            self.buffer[self.cursor_line].pop();
        } else if self.buffer.len() > 1 {
            // Remove the current line and move to the one above (or stay on the first line)
            self.buffer.remove(self.cursor_line);
            self.cursor_line = self.cursor_line.saturating_sub(1);
        }
    }

    pub fn clear(&mut self) {
        self.buffer = vec![String::new()];
        self.cursor_line = 0;
    }

    pub fn cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor_line < self.buffer.len() - 1 {
            self.cursor_line += 1;
        }
    }

    /// Start a new line below the current one
    pub fn new_line(&mut self) {
        self.cursor_line += 1;
        self.buffer.insert(self.cursor_line, String::new());
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_new_creates_empty_buffer() {
        let editor = Editor::new();
        assert_eq!(editor.message(), "");
        assert_eq!(editor.cursor_line, 0);
        assert_eq!(editor.buffer.len(), 1);
    }

    #[test]
    fn editor_default_creates_empty_buffer() {
        let editor = Editor::default();
        assert_eq!(editor.message(), "");
    }

    #[test]
    fn editor_insert_single_char() {
        let mut editor = Editor::new();
        editor.insert('a');
        assert_eq!(editor.message(), "a");
    }

    #[test]
    fn editor_insert_multiple_chars() {
        let mut editor = Editor::new();
        editor.insert('h');
        editor.insert('e');
        editor.insert('l');
        editor.insert('l');
        editor.insert('o');
        assert_eq!(editor.message(), "hello");
    }

    #[test]
    fn editor_insert_unicode_chars() {
        let mut editor = Editor::new();
        editor.insert('こ');
        editor.insert('ん');
        editor.insert('に');
        editor.insert('ち');
        editor.insert('は');
        assert_eq!(editor.message(), "こんにちは");
    }

    #[test]
    fn editor_new_line_creates_multiline() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        editor.insert('b');
        assert_eq!(editor.message(), "a\nb");
        assert_eq!(editor.cursor_line, 1);
        assert_eq!(editor.buffer.len(), 2);
    }

    #[test]
    fn editor_new_line_multiple_lines() {
        let mut editor = Editor::new();
        editor.insert('1');
        editor.new_line();
        editor.insert('2');
        editor.new_line();
        editor.insert('3');
        assert_eq!(editor.message(), "1\n2\n3");
        assert_eq!(editor.cursor_line, 2);
        assert_eq!(editor.buffer.len(), 3);
    }

    #[test]
    fn editor_backspace_removes_char() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.insert('b');
        editor.backspace();
        assert_eq!(editor.message(), "a");
    }

    #[test]
    fn editor_backspace_on_empty_line_does_nothing() {
        let mut editor = Editor::new();
        editor.backspace();
        assert_eq!(editor.message(), "");
        assert_eq!(editor.buffer.len(), 1);
    }

    #[test]
    fn editor_backspace_removes_empty_line() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        // Now on line 2 (empty), backspace should remove this line
        editor.backspace();
        assert_eq!(editor.message(), "a");
        assert_eq!(editor.buffer.len(), 1);
        assert_eq!(editor.cursor_line, 0);
    }

    #[test]
    fn editor_backspace_removes_empty_first_line() {
        let mut editor = Editor::new();
        editor.new_line();
        editor.insert('a');
        editor.cursor_up();
        // The first line is empty: remove it and stay on the first line
        editor.backspace();
        assert_eq!(editor.message(), "a");
        assert_eq!(editor.cursor_line, 0);
    }

    #[test]
    fn editor_new_line_after_cursor_up() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        editor.insert('b');
        editor.cursor_up();
        editor.new_line();
        editor.insert('c');
        assert_eq!(editor.message(), "a\nc\nb");
        assert_eq!(editor.cursor_line, 1);
    }

    #[test]
    fn editor_draw_message_clears_removed_lines() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        editor.backspace();
        let mut stdout = Vec::new();
        editor.draw_message(&mut stdout);
        let out = String::from_utf8(stdout).unwrap();
        assert_eq!(
            out,
            format!(
                "{}{}a{}",
                termion::cursor::Goto(1, MESSAGE_ROW),
                termion::clear::AfterCursor,
                termion::cursor::Goto(2, MESSAGE_ROW)
            )
        );
    }

    #[test]
    fn editor_draw_message_puts_cursor_after_wide_chars() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        editor.insert('こ');
        editor.insert('ん');
        let mut stdout = Vec::new();
        editor.draw_message(&mut stdout);
        let out = String::from_utf8(stdout).unwrap();
        // "こん" is 4 columns wide, on the second message line
        assert!(out.contains("a\r\nこん"));
        assert!(out.ends_with(&termion::cursor::Goto(5, MESSAGE_ROW + 1).to_string()));
    }

    #[test]
    fn editor_cursor_up_moves_cursor() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        editor.insert('b');
        assert_eq!(editor.cursor_line, 1);
        editor.cursor_up();
        assert_eq!(editor.cursor_line, 0);
    }

    #[test]
    fn editor_cursor_up_at_top_stays_at_top() {
        let mut editor = Editor::new();
        editor.insert('a');
        assert_eq!(editor.cursor_line, 0);
        editor.cursor_up();
        assert_eq!(editor.cursor_line, 0);
    }

    #[test]
    fn editor_cursor_down_moves_cursor() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        editor.insert('b');
        editor.cursor_up();
        assert_eq!(editor.cursor_line, 0);
        editor.cursor_down();
        assert_eq!(editor.cursor_line, 1);
    }

    #[test]
    fn editor_cursor_down_at_bottom_stays_at_bottom() {
        let mut editor = Editor::new();
        editor.insert('a');
        assert_eq!(editor.cursor_line, 0);
        editor.cursor_down();
        assert_eq!(editor.cursor_line, 0);
    }

    #[test]
    fn editor_clear_resets_buffer() {
        let mut editor = Editor::new();
        editor.insert('a');
        editor.new_line();
        editor.insert('b');
        editor.clear();
        assert_eq!(editor.message(), "");
        assert_eq!(editor.buffer.len(), 1);
        assert_eq!(editor.cursor_line, 0);
    }
}
