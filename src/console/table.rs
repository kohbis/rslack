use std::io::Write;

use termion::color;
use termion::terminal_size;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const BAR: &str = "|";
const WHITESPACE: &str = " ";
const HYPHEN: &str = "-";
const CORNER: &str = "+";
const USAGE_TABLE: &str = "Select by ← ↓ ↑ → or h j k l, and Enter. / filters.";

pub struct Table {
    name: String,
    data: Vec<String>,
    max_col_size: usize,
}

/// Horizontal border for columns of the given widths, e.g. "+---+-----+"
pub(crate) fn border(widths: &[usize]) -> String {
    let rules: Vec<String> = widths.iter().map(|width| HYPHEN.repeat(*width)).collect();
    format!("{}{}{}", CORNER, rules.join(CORNER), CORNER)
}

/// Split a line into pieces of at most `width` display columns
pub(crate) fn wrap(line: &str, width: usize) -> Vec<String> {
    let mut pieces = vec![];
    let mut piece = String::new();
    let mut piece_width = 0;
    for c in line.replace('\t', "    ").chars() {
        // Skip control characters, which have no width and break the alignment
        let Some(char_width) = c.width() else {
            continue;
        };
        if piece_width + char_width > width {
            pieces.push(std::mem::take(&mut piece));
            piece_width = 0;
        }
        piece.push(c);
        piece_width += char_width;
    }
    pieces.push(piece);
    pieces
}

/// Cut `text` to `width` display columns, marking the cut with "…"
pub(crate) fn truncate(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut truncated = String::new();
    let mut truncated_width = 0;
    for c in text.chars() {
        let char_width = c.width().unwrap_or(0);
        if truncated_width + char_width > width - 1 {
            break;
        }
        truncated.push(c);
        truncated_width += char_width;
    }
    truncated.push('…');
    truncated
}

/// Pad `text` with spaces to `width` display columns
pub(crate) fn pad(text: &str, width: usize) -> String {
    format!("{}{}", text, " ".repeat(width.saturating_sub(text.width())))
}

/*
 * Get terminal window size.
 */
pub(crate) fn term_size() -> (u16, u16) {
    match terminal_size() {
        Ok((width, height)) => (width, height),
        _ => (100, 100),
    }
}

impl Table {
    pub fn new(name: String, data: Vec<String>, max_col_size: usize) -> Table {
        Table {
            name,
            data,
            max_col_size,
        }
    }

    /// Channels per row; at least one, even when a name is wider than the terminal
    pub fn col_count(&self) -> usize {
        (term_size().0 as usize / (self.max_col_size + 2)).max(1)
    }

    pub fn chunked_data(&self) -> Vec<Vec<String>> {
        self.data
            .chunks(self.col_count())
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /*
     * Print data as table.
     */
    pub fn draw(&self, stdout: &mut dyn Write, selected: &str) {
        let rows = self.chunked_data();
        self.draw_rows(stdout, &rows, selected);
    }

    /// Print `rows` as they are, without chunking again from the terminal width.
    pub fn draw_rows(&self, stdout: &mut dyn Write, rows: &[Vec<String>], selected: &str) {
        write!(
            stdout,
            "{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::All
        )
        .unwrap();

        if rows.is_empty() {
            stdout.flush().unwrap();
            return;
        }

        let painted: Vec<Vec<String>> = rows
            .iter()
            .map(|names| {
                names
                    .iter()
                    .map(|cell| {
                        // Highlight selected channel
                        let (fg_color, bg_color) = if cell == selected {
                            (
                                color::Fg(color::Black).to_string(),
                                color::Bg(color::White).to_string(),
                            )
                        } else {
                            (
                                color::Fg(color::Reset).to_string(),
                                color::Bg(color::Reset).to_string(),
                            )
                        };

                        format!(
                            "{}{}{}{}{}{}",
                            bg_color,
                            WHITESPACE,
                            fg_color,
                            pad(cell, self.max_col_size),
                            color::Fg(color::Reset),
                            color::Bg(color::Reset),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // Each cell is a leading space and the padded channel name
        let row_border = |row: &Vec<String>| border(&vec![self.max_col_size + 1; row.len()]);
        self.print_head_channels(stdout, &row_border(&painted[0]));
        for row in &painted {
            Self::print_row(stdout, &row.join(BAR));
            Self::print_line(stdout, &row_border(row));
        }
        write!(stdout, "{}", USAGE_TABLE).unwrap();
        stdout.flush().unwrap()
    }

    /*
     * Print table row with bar.
     */
    fn print_row(stdout: &mut dyn Write, content: &str) {
        Self::print_line(stdout, &format!("{}{}{}", BAR, content, BAR));
    }

    fn print_line(stdout: &mut dyn Write, line: &str) {
        write!(stdout, "{}\r\n", line).unwrap();
    }

    /*
     * Print table header, followed by `first_row_border`.
     */
    fn print_head_channels(&self, stdout: &mut dyn Write, first_row_border: &str) {
        let size = first_row_border.len() - 2;
        let margin = size - self.name.len();
        let margin_left = margin / 2;
        let margin_right = margin - margin_left;

        let head = [
            &WHITESPACE.repeat(margin_left),
            self.name.as_str(),
            &WHITESPACE.repeat(margin_right),
        ]
        .concat();

        Self::print_line(stdout, &border(&[size]));
        Self::print_row(stdout, &head);
        Self::print_line(stdout, first_row_border);
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    fn strip_escapes(out: &[u8]) -> String {
        let out = String::from_utf8(out.to_vec()).unwrap();
        out.split('\x1b')
            .map(|part| match part.find(|c: char| c.is_ascii_alphabetic()) {
                Some(i) if part.starts_with('[') => &part[i + 1..],
                _ => part,
            })
            .collect()
    }

    #[test]
    fn build_table() {
        let name = "test".to_string();
        let data = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let max_col_size = 1;
        let table = Table::new(name, data, max_col_size);
        assert_eq!(table.name, "test".to_string());
        assert_eq!(
            table.data,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(table.max_col_size, 1);
    }

    #[test]
    fn wrap_by_display_width() {
        assert_eq!(wrap("abcdef", 3), ["abc", "def"]);
        assert_eq!(wrap("あいう", 4), ["あい", "う"]);
        assert_eq!(wrap("", 3), [""]);
        assert_eq!(wrap("a\tb", 3), ["a  ", "  b"]);
    }

    #[test]
    fn truncate_by_display_width() {
        assert_eq!(truncate("alice", 5), "alice");
        assert_eq!(truncate("alice.smith", 5), "alic…");
        assert_eq!(truncate("あいうえお", 5), "あい…");
    }

    #[test]
    fn pad_by_display_width() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("あ", 4), "あ  ");
        assert_eq!(pad("abcde", 4), "abcde");
    }

    #[test]
    fn chunked_data_with_column_wider_than_terminal() {
        let data = vec!["a".to_string(), "b".to_string()];
        let table = Table::new("test".to_string(), data, u16::MAX as usize);
        assert_eq!(table.col_count(), 1);
        assert_eq!(table.chunked_data(), [["a"], ["b"]]);
    }

    #[test]
    fn border_with_widths() {
        assert_eq!(border(&[5]), "+-----+");
        assert_eq!(border(&[2, 3]), "+--+---+");
    }

    #[test]
    fn draw_rows_keeps_the_given_layout() {
        let table = Table::new(
            "CH".to_string(),
            ["a", "b", "c"].map(String::from).to_vec(),
            3,
        );
        let mut out = vec![];
        table.draw_rows(
            &mut out,
            &[
                vec!["a".to_string(), "b".to_string()],
                vec!["c".to_string()],
            ],
            "b",
        );
        let plain = strip_escapes(&out);
        let lines: Vec<&str> = plain.lines().collect();
        assert!(lines
            .iter()
            .any(|line| line.contains('a') && line.contains('b')));
        assert!(lines
            .iter()
            .any(|line| line.contains('c') && !line.contains('a')));
    }

    #[test]
    fn draw_channels_with_borders() {
        let data = ["a", "bb", "ccc"].map(String::from).to_vec();
        let table = Table::new("CH".to_string(), data, 3);
        let mut out = vec![];
        table.draw(&mut out, "bb");
        let plain = strip_escapes(&out);
        assert_eq!(
            plain,
            [
                "+--------------+",
                "|      CH      |",
                "+----+----+----+",
                "| a  | bb | ccc|",
                "+----+----+----+",
                USAGE_TABLE,
            ]
            .join("\r\n")
        );
    }
}
