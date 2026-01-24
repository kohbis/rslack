use std::io::Write;

const USAGE_EDITOR: &str = "(post: ctrl-p / exit: ctrl-c)";

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
        self.buffer.join("\r\n")
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
            termion::cursor::Goto(1, 3)
        )
        .unwrap();
        stdout.flush().unwrap();
    }

    pub fn draw_message(&mut self, stdout: &mut dyn Write) {
        write!(
            stdout,
            "{}{}{}",
            termion::cursor::Goto(1, 3),
            termion::clear::CurrentLine,
            self.message()
        )
        .unwrap();
        write!(
            stdout,
            "{}",
            termion::cursor::Goto(
                self.buffer[self.cursor_line].len() as u16 + 1,
                self.cursor_line as u16 + 3
            )
        )
        .unwrap();
        stdout.flush().unwrap();
    }

    pub fn insert(&mut self, c: char) {
        self.buffer[self.cursor_line].push(c);
    }

    pub fn backspace(&mut self, stdout: &mut dyn Write) {
        if !self.buffer[self.cursor_line].is_empty() {
            self.buffer[self.cursor_line].pop();
            write!(
                stdout,
                "{}{}",
                termion::cursor::Left(1),
                termion::clear::AfterCursor
            )
            .unwrap();
        } else if self.buffer.len() > 1 {
            // Remove current line
            self.buffer.remove(self.cursor_line);
            self.cursor_line -= 1;
        }
    }

    pub fn clear(&mut self, stdout: &mut dyn Write) {
        self.buffer = vec![String::new()];
        self.cursor_line = 0;

        write!(
            stdout,
            "{}{}",
            termion::cursor::Goto(1, 3),
            termion::clear::CurrentLine
        )
        .unwrap();
        stdout.flush().unwrap();
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

    pub fn new_line(&mut self) {
        self.buffer.push(String::new());
        self.cursor_line += 1;
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
        assert_eq!(editor.message(), "a\r\nb");
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
        assert_eq!(editor.message(), "1\r\n2\r\n3");
        assert_eq!(editor.cursor_line, 2);
        assert_eq!(editor.buffer.len(), 3);
    }

    #[test]
    fn editor_backspace_removes_char() {
        let mut editor = Editor::new();
        let mut stdout = Vec::new();
        editor.insert('a');
        editor.insert('b');
        editor.backspace(&mut stdout);
        assert_eq!(editor.message(), "a");
    }

    #[test]
    fn editor_backspace_on_empty_line_does_nothing() {
        let mut editor = Editor::new();
        let mut stdout = Vec::new();
        editor.backspace(&mut stdout);
        assert_eq!(editor.message(), "");
        assert_eq!(editor.buffer.len(), 1);
    }

    #[test]
    fn editor_backspace_removes_empty_line() {
        let mut editor = Editor::new();
        let mut stdout = Vec::new();
        editor.insert('a');
        editor.new_line();
        // Now on line 2 (empty), backspace should remove this line
        editor.backspace(&mut stdout);
        assert_eq!(editor.message(), "a");
        assert_eq!(editor.buffer.len(), 1);
        assert_eq!(editor.cursor_line, 0);
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
        let mut stdout = Vec::new();
        editor.insert('a');
        editor.new_line();
        editor.insert('b');
        editor.clear(&mut stdout);
        assert_eq!(editor.message(), "");
        assert_eq!(editor.buffer.len(), 1);
        assert_eq!(editor.cursor_line, 0);
    }
}
