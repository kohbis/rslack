use std::io::{Read, Write};

use anyhow::Result;
use termion::event::Key;
use termion::input::TermRead;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
    /// Cursor position in the current line, counted in chars (not bytes)
    cursor_col: usize,
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            buffer: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
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
                Key::Char('\n') => self.new_line(),
                Key::Char(c) => self.insert(c),
                Key::Up => self.cursor_up(),
                Key::Down => self.cursor_down(),
                Key::Left => self.cursor_left(),
                Key::Right => self.cursor_right(),
                Key::Home | Key::Ctrl('a') => self.cursor_home(),
                Key::End | Key::Ctrl('e') => self.cursor_end(),
                Key::Backspace => self.backspace(),
                Key::Delete => self.delete(),
                _ => {}
            }

            self.draw_message(stdout);
        }

        Ok(EditorResult::Submitted(self.message()))
    }

    pub fn draw_header(&self, stdout: &mut dyn Write, channel: &str) {
        write!(
            stdout,
            "{}{}{}{}{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All,
            super::message_viewer::channel_heading(channel),
            termion::cursor::Goto(1, 2),
            USAGE_EDITOR,
            termion::cursor::Goto(1, MESSAGE_ROW)
        )
        .unwrap();
        stdout.flush().unwrap();
    }

    /// Redraw the whole message, clearing what was below it (e.g. a removed line),
    /// and put the cursor at its position
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
        let col = self.cursor_width() as u16 + 1;
        let row = self.cursor_line as u16 + MESSAGE_ROW;
        write!(stdout, "{}", termion::cursor::Goto(col, row)).unwrap();
        stdout.flush().unwrap();
    }

    /// Insert a character at the cursor
    pub fn insert(&mut self, c: char) {
        let index = self.byte_index();
        self.buffer[self.cursor_line].insert(index, c);
        self.cursor_col += 1;
    }

    /// Delete the character before the cursor, joining with the previous line at its start
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let index = self.byte_index();
            self.buffer[self.cursor_line].remove(index);
        } else if self.cursor_line > 0 {
            let line = self.buffer.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.line_len();
            self.buffer[self.cursor_line].push_str(&line);
        }
    }

    /// Delete the character at the cursor, joining with the next line at its end
    pub fn delete(&mut self) {
        if self.cursor_col < self.line_len() {
            let index = self.byte_index();
            self.buffer[self.cursor_line].remove(index);
        } else if self.cursor_line + 1 < self.buffer.len() {
            let line = self.buffer.remove(self.cursor_line + 1);
            self.buffer[self.cursor_line].push_str(&line);
        }
    }

    pub fn clear(&mut self) {
        self.buffer = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
    }

    /// Move up a line, keeping the cursor at about the same screen column
    pub fn cursor_up(&mut self) {
        if self.cursor_line > 0 {
            let width = self.cursor_width();
            self.cursor_line -= 1;
            self.cursor_col = self.col_at_width(width);
        }
    }

    /// Move down a line, keeping the cursor at about the same screen column
    pub fn cursor_down(&mut self) {
        if self.cursor_line + 1 < self.buffer.len() {
            let width = self.cursor_width();
            self.cursor_line += 1;
            self.cursor_col = self.col_at_width(width);
        }
    }

    /// Move left a character, to the end of the previous line at the start of a line
    pub fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.line_len();
        }
    }

    /// Move right a character, to the start of the next line at the end of a line
    pub fn cursor_right(&mut self) {
        if self.cursor_col < self.line_len() {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.buffer.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    pub fn cursor_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn cursor_end(&mut self) {
        self.cursor_col = self.line_len();
    }

    /// Split the line at the cursor, moving the rest to a new line below
    pub fn new_line(&mut self) {
        let index = self.byte_index();
        let rest = self.buffer[self.cursor_line].split_off(index);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.buffer.insert(self.cursor_line, rest);
    }

    /// Number of chars in the current line
    fn line_len(&self) -> usize {
        self.buffer[self.cursor_line].chars().count()
    }

    /// Byte index in the current line for the cursor's char position
    fn byte_index(&self) -> usize {
        let line = &self.buffer[self.cursor_line];
        line.char_indices()
            .nth(self.cursor_col)
            .map_or(line.len(), |(index, _)| index)
    }

    /// Display width of the current line before the cursor
    fn cursor_width(&self) -> usize {
        self.buffer[self.cursor_line][..self.byte_index()].width()
    }

    /// Char position in the current line that is closest to, but not past, `width` columns
    fn col_at_width(&self, width: usize) -> usize {
        let mut current = 0;
        let mut col = 0;
        for c in self.buffer[self.cursor_line].chars() {
            current += c.width().unwrap_or(0);
            if current > width {
                break;
            }
            col += 1;
        }
        col
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
    fn editor_backspace_at_start_does_nothing() {
        let mut editor = Editor::new();
        editor.new_line();
        editor.insert('a');
        editor.cursor_up();
        // At the very start there is nothing before the cursor (and no panic)
        editor.backspace();
        assert_eq!(editor.message(), "\na");
        assert_eq!((editor.cursor_line, editor.cursor_col), (0, 0));
    }

    /// Editor with `text` typed in, cursor at the end
    fn editor_with(text: &str) -> Editor {
        let mut editor = Editor::new();
        for c in text.chars() {
            match c {
                '\n' => editor.new_line(),
                c => editor.insert(c),
            }
        }
        editor
    }

    #[test]
    fn editor_insert_in_the_middle() {
        let mut editor = editor_with("helo");
        editor.cursor_left();
        editor.insert('l');
        assert_eq!(editor.message(), "hello");
        assert_eq!(editor.cursor_col, 4);
    }

    #[test]
    fn editor_backspace_in_the_middle_of_wide_chars() {
        let mut editor = editor_with("こんにちは");
        editor.cursor_left();
        editor.cursor_left();
        editor.backspace();
        assert_eq!(editor.message(), "こんちは");
        assert_eq!(editor.cursor_col, 2);
    }

    #[test]
    fn editor_backspace_at_line_start_joins_lines() {
        let mut editor = editor_with("ab\ncd");
        editor.cursor_home();
        editor.backspace();
        assert_eq!(editor.message(), "abcd");
        assert_eq!((editor.cursor_line, editor.cursor_col), (0, 2));
    }

    #[test]
    fn editor_delete_at_cursor() {
        let mut editor = editor_with("abc");
        editor.cursor_home();
        editor.delete();
        assert_eq!(editor.message(), "bc");
        assert_eq!(editor.cursor_col, 0);
    }

    #[test]
    fn editor_delete_at_line_end_joins_lines() {
        let mut editor = editor_with("ab\ncd");
        editor.cursor_up();
        editor.delete();
        assert_eq!(editor.message(), "abcd");
        assert_eq!((editor.cursor_line, editor.cursor_col), (0, 2));
        // Nothing after the last character
        editor.cursor_end();
        editor.delete();
        assert_eq!(editor.message(), "abcd");
    }

    #[test]
    fn editor_new_line_splits_line() {
        let mut editor = editor_with("abcd");
        editor.cursor_left();
        editor.cursor_left();
        editor.new_line();
        assert_eq!(editor.message(), "ab\ncd");
        assert_eq!((editor.cursor_line, editor.cursor_col), (1, 0));
    }

    #[test]
    fn editor_cursor_left_right_across_lines() {
        let mut editor = editor_with("ab\ncd");
        editor.cursor_home();
        editor.cursor_left();
        assert_eq!((editor.cursor_line, editor.cursor_col), (0, 2));
        editor.cursor_right();
        assert_eq!((editor.cursor_line, editor.cursor_col), (1, 0));
        // Stays put at the very start and end
        editor.cursor_up();
        editor.cursor_home();
        editor.cursor_left();
        assert_eq!((editor.cursor_line, editor.cursor_col), (0, 0));
        editor.cursor_down();
        editor.cursor_end();
        editor.cursor_right();
        assert_eq!((editor.cursor_line, editor.cursor_col), (1, 2));
    }

    #[test]
    fn editor_cursor_up_down_keeps_screen_column() {
        // "こんにちは" is 10 columns wide; "abcdef" is 6
        let mut editor = editor_with("こんにちは\nabcdef");
        editor.cursor_left();
        editor.cursor_left();
        // Column 4 in "abcdef" is at the boundary after "こん"
        editor.cursor_up();
        assert_eq!(editor.cursor_col, 2);
        editor.cursor_right();
        // Column 6 after "こんに" maps back to the end of "abcdef"
        editor.cursor_down();
        assert_eq!(editor.cursor_col, 6);
        // Column 5 falls in the middle of "に": stay before it
        let mut editor = editor_with("こんにちは\nabcde");
        editor.cursor_up();
        assert_eq!(editor.cursor_col, 2);
        // A shorter line clamps to its end
        let mut editor = editor_with("ab\nabcdef");
        editor.cursor_up();
        assert_eq!(editor.cursor_col, 2);
    }

    #[test]
    fn editor_draw_message_puts_cursor_in_the_middle() {
        let mut editor = editor_with("aこんb");
        editor.cursor_left();
        editor.cursor_left();
        let mut stdout = Vec::new();
        editor.draw_message(&mut stdout);
        let out = String::from_utf8(stdout).unwrap();
        // The cursor sits after "aこ", 3 columns wide
        assert!(out.ends_with(&termion::cursor::Goto(4, MESSAGE_ROW).to_string()));
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
