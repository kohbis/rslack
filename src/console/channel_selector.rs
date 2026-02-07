use std::io::{Read, Write};

use anyhow::Result;
use termion::event::Key;
use termion::input::TermRead;

use super::Table;

/// Interactive channel selector with vim-like navigation
pub struct ChannelSelector {
    table: Table,
    channel_names: Vec<String>,
}

/// Result of channel selection
pub enum SelectionResult {
    /// User selected a channel
    Selected(String),
    /// User cancelled the selection (q or Ctrl-C)
    Cancelled,
}

impl ChannelSelector {
    pub fn new(channel_names: Vec<String>, max_col_size: usize) -> Self {
        let table = Table::new("CHANNELS".to_string(), channel_names.clone(), max_col_size);
        Self {
            table,
            channel_names,
        }
    }

    /// Check if channel selection is needed
    pub fn needs_selection(&self, channel: &str) -> bool {
        channel.trim().is_empty() || !self.channel_names.contains(&channel.to_string())
    }

    /// Run the interactive channel selection
    /// Returns SelectionResult::Selected(channel_name) if selected, SelectionResult::Cancelled if cancelled
    pub fn run<R: Read, W: Write>(&self, stdin: R, stdout: &mut W) -> Result<SelectionResult> {
        let chunked_data = self.table.chunked_data();
        let (row_lengths, row_starts, total_items) = Self::row_metadata(&chunked_data);
        if total_items == 0 {
            return Ok(SelectionResult::Cancelled);
        }
        let flat: Vec<String> = chunked_data
            .iter()
            .flat_map(|row| row.iter().cloned())
            .collect();
        let mut index = 0usize;

        let mut selected = flat[index].to_string();
        self.table.draw(stdout, &selected);

        for c in stdin.keys() {
            match c? {
                Key::Char('q') | Key::Ctrl('c') => return Ok(SelectionResult::Cancelled),
                Key::Char('\n') => return Ok(SelectionResult::Selected(selected)),
                Key::Left | Key::Char('h') => {
                    index = Self::horizontal_index(index, &row_lengths, &row_starts, false);
                }
                Key::Right | Key::Char('l') => {
                    index = Self::horizontal_index(index, &row_lengths, &row_starts, true);
                }
                Key::Up | Key::Char('k') => {
                    index = Self::vertical_index(index, &row_lengths, &row_starts, false);
                }
                Key::Down | Key::Char('j') => {
                    index = Self::vertical_index(index, &row_lengths, &row_starts, true);
                }
                _ => {}
            }

            selected = flat[index].to_string();
            self.table.draw(stdout, &selected);
        }

        Ok(SelectionResult::Selected(selected))
    }

    /// Draw the table with the given channel selected
    pub fn draw<W: Write>(&self, stdout: &mut W, selected: &str) {
        self.table.draw(stdout, selected);
    }

    fn row_metadata(chunked_data: &[Vec<String>]) -> (Vec<usize>, Vec<usize>, usize) {
        let mut row_lengths = Vec::with_capacity(chunked_data.len());
        let mut row_starts = Vec::with_capacity(chunked_data.len());
        let mut total = 0usize;

        for row in chunked_data {
            row_starts.push(total);
            let len = row.len();
            if len == 0 {
                row_starts.pop();
                continue;
            }
            row_lengths.push(len);
            total += len;
        }

        (row_lengths, row_starts, total)
    }

    fn horizontal_index(
        current: usize,
        row_lengths: &[usize],
        row_starts: &[usize],
        moving_right: bool,
    ) -> usize {
        let (row, col) = Self::row_col_for_index(current, row_lengths, row_starts);
        let row_len = row_lengths[row];
        let next_col = if moving_right {
            if col + 1 == row_len {
                0
            } else {
                col + 1
            }
        } else if col == 0 {
            row_len - 1
        } else {
            col - 1
        };

        Self::index_for_row_col(row, next_col, row_starts)
    }

    fn row_col_for_index(
        index: usize,
        row_lengths: &[usize],
        row_starts: &[usize],
    ) -> (usize, usize) {
        let mut row = 0usize;
        for (i, start) in row_starts.iter().enumerate() {
            if index >= *start {
                row = i;
            } else {
                break;
            }
        }
        let col = (index - row_starts[row]).min(row_lengths[row] - 1);
        (row, col)
    }

    fn index_for_row_col(row: usize, col: usize, row_starts: &[usize]) -> usize {
        row_starts[row] + col
    }

    fn vertical_index(
        current: usize,
        row_lengths: &[usize],
        row_starts: &[usize],
        moving_down: bool,
    ) -> usize {
        if row_lengths.is_empty() {
            return current;
        }
        let (current_row, current_col) = Self::row_col_for_index(current, row_lengths, row_starts);
        let row_count = row_lengths.len();
        let next_row = if moving_down {
            (current_row + 1) % row_count
        } else if current_row == 0 {
            row_count - 1
        } else {
            current_row - 1
        };

        if current_col >= row_lengths[next_row] {
            if moving_down {
                if row_lengths[0] > current_col {
                    return Self::index_for_row_col(0, current_col, row_starts);
                }
            } else if current_row == 0 {
                // From top row, if last row doesn't have this column, stay put.
                return current;
            }
        }

        let next_col = current_col.min(row_lengths[next_row] - 1);

        Self::index_for_row_col(next_row, next_col, row_starts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_new_creates_instance() {
        let channels = vec!["general".to_string(), "random".to_string()];
        let selector = ChannelSelector::new(channels, 10);
        assert_eq!(selector.channel_names.len(), 2);
    }

    #[test]
    fn needs_selection_with_empty_channel() {
        let channels = vec!["general".to_string(), "random".to_string()];
        let selector = ChannelSelector::new(channels, 10);
        assert!(selector.needs_selection(""));
        assert!(selector.needs_selection("   "));
    }

    #[test]
    fn needs_selection_with_invalid_channel() {
        let channels = vec!["general".to_string(), "random".to_string()];
        let selector = ChannelSelector::new(channels, 10);
        assert!(selector.needs_selection("nonexistent"));
    }

    #[test]
    fn needs_selection_with_valid_channel() {
        let channels = vec!["general".to_string(), "random".to_string()];
        let selector = ChannelSelector::new(channels, 10);
        assert!(!selector.needs_selection("general"));
        assert!(!selector.needs_selection("random"));
    }

    #[test]
    fn horizontal_left_right_wraps_within_row() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["d".to_string()],
        ];
        let (lengths, starts, _) = ChannelSelector::row_metadata(&rows);

        let index_b = ChannelSelector::index_for_row_col(0, 1, &starts);
        let left = ChannelSelector::horizontal_index(index_b, &lengths, &starts, false);
        let (row_left, col_left) = ChannelSelector::row_col_for_index(left, &lengths, &starts);
        assert_eq!((row_left, col_left), (0, 0));

        let index_a = ChannelSelector::index_for_row_col(0, 0, &starts);
        let wrap_left = ChannelSelector::horizontal_index(index_a, &lengths, &starts, false);
        let (row_wrap_left, col_wrap_left) =
            ChannelSelector::row_col_for_index(wrap_left, &lengths, &starts);
        assert_eq!((row_wrap_left, col_wrap_left), (0, 2));

        let index_c = ChannelSelector::index_for_row_col(0, 2, &starts);
        let wrap_right = ChannelSelector::horizontal_index(index_c, &lengths, &starts, true);
        let (row_wrap_right, col_wrap_right) =
            ChannelSelector::row_col_for_index(wrap_right, &lengths, &starts);
        assert_eq!((row_wrap_right, col_wrap_right), (0, 0));

        let index_d = ChannelSelector::index_for_row_col(1, 0, &starts);
        let right_same_row = ChannelSelector::horizontal_index(index_d, &lengths, &starts, true);
        let (row_right, col_right) =
            ChannelSelector::row_col_for_index(right_same_row, &lengths, &starts);
        assert_eq!((row_right, col_right), (1, 0));
    }

    #[test]
    fn linear_vertical_clamps_to_row_length() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["d".to_string(), "e".to_string()],
            vec!["f".to_string(), "g".to_string(), "h".to_string()],
        ];
        let (lengths, starts, _) = ChannelSelector::row_metadata(&rows);

        let index_c = ChannelSelector::index_for_row_col(0, 2, &starts);
        let down_from_c = ChannelSelector::vertical_index(index_c, &lengths, &starts, true);
        let (row, col) = ChannelSelector::row_col_for_index(down_from_c, &lengths, &starts);
        assert_eq!((row, col), (0, 2));

        let up_from_clamped = ChannelSelector::vertical_index(down_from_c, &lengths, &starts, false);
        let (row_up, col_up) = ChannelSelector::row_col_for_index(up_from_clamped, &lengths, &starts);
        assert_eq!((row_up, col_up), (2, 2));
    }

    #[test]
    fn moving_down_from_short_row_wraps_to_top_same_column() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["d".to_string(), "e".to_string(), "f".to_string()],
            vec!["g".to_string(), "h".to_string()],
        ];
        let (lengths, starts, _) = ChannelSelector::row_metadata(&rows);

        let index_col2 = ChannelSelector::index_for_row_col(1, 2, &starts);
        let down = ChannelSelector::vertical_index(index_col2, &lengths, &starts, true);
        let (row, col) = ChannelSelector::row_col_for_index(down, &lengths, &starts);
        assert_eq!((row, col), (0, 2));
    }

    #[test]
    fn moving_up_from_short_row_clamps_to_last_column() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string(), "e".to_string()],
            vec!["f".to_string(), "g".to_string(), "h".to_string()],
        ];
        let (lengths, starts, _) = ChannelSelector::row_metadata(&rows);

        let index_col2 = ChannelSelector::index_for_row_col(1, 2, &starts);
        let up = ChannelSelector::vertical_index(index_col2, &lengths, &starts, false);
        let (row, col) = ChannelSelector::row_col_for_index(up, &lengths, &starts);
        assert_eq!((row, col), (0, 1));
    }

    #[test]
    fn moving_up_from_top_row_short_last_row_stays_put() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["d".to_string(), "e".to_string(), "f".to_string()],
            vec!["g".to_string(), "h".to_string()],
        ];
        let (lengths, starts, _) = ChannelSelector::row_metadata(&rows);

        let index_col2 = ChannelSelector::index_for_row_col(0, 2, &starts);
        let up = ChannelSelector::vertical_index(index_col2, &lengths, &starts, false);
        let (row, col) = ChannelSelector::row_col_for_index(up, &lengths, &starts);
        assert_eq!((row, col), (0, 2));
    }

    #[test]
    fn row_metadata_skips_empty_rows() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string()],
            vec![],
            vec!["c".to_string()],
        ];
        let (lengths, starts, total) = ChannelSelector::row_metadata(&rows);

        assert_eq!(total, 3);
        assert_eq!(lengths, vec![2, 1]);
        assert_eq!(starts, vec![0, 2]);
    }
}
