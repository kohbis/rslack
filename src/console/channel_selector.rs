use std::io::{Read, Write};

use anyhow::Result;
use rpos::table::Table as RposTable;
use rpos::WrapMode;
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

        let widths = self
            .table
            .chunked_data()
            .iter()
            .map(|row| row.len())
            .collect::<Vec<usize>>();
        let num_rows = widths.len();
        let mut cursor = RposTable::new_jagged(widths.clone())?
            .wrap_mode(WrapMode::Wrap)
            .cursor;

        let mut selected = chunked_data[cursor.current().0][cursor.current().1].to_string();
        self.table.draw(stdout, &selected);

        for c in stdin.keys() {
            match c? {
                Key::Char('q') | Key::Ctrl('c') => return Ok(SelectionResult::Cancelled),
                Key::Char('\n') => return Ok(SelectionResult::Selected(selected.to_string())),
                Key::Left | Key::Char('h') => {
                    cursor.left();
                }
                Key::Right | Key::Char('l') => {
                    cursor.right();
                }
                Key::Up | Key::Char('k') => {
                    let (row, col) = cursor.current();
                    // NOTE: Skip rows that don't have the current column
                    //       rpos's cursor.up() does not handle jagged tables well in this case
                    for i in 1..num_rows {
                        let target = (row + num_rows - i) % num_rows;
                        if widths[target] > col {
                            let _ = cursor.set(target, col);
                            break;
                        }
                    }
                }
                Key::Down | Key::Char('j') => {
                    let (row, col) = cursor.current();
                    // NOTE: Skip rows that don't have the current column
                    //       rpos's cursor.down() does not handle jagged tables well in this case
                    for i in 1..num_rows {
                        let target = (row + i) % num_rows;
                        if widths[target] > col {
                            let _ = cursor.set(target, col);
                            break;
                        }
                    }
                }
                _ => {}
            }

            selected = chunked_data[cursor.current().0][cursor.current().1].to_string();
            self.table.draw(stdout, &selected);
        }

        Ok(SelectionResult::Selected(selected))
    }

    /// Draw the table with the given channel selected
    pub fn draw<W: Write>(&self, stdout: &mut W, selected: &str) {
        self.table.draw(stdout, selected);
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
}
