use std::io::{Read, Write};

use anyhow::{bail, Result};
use rpos::table::Table as RposTable;
use rpos::{VerticalPolicy, WrapMode};
use termion::event::Key;
use termion::input::TermRead;

use super::Table;

/// Interactive channel selector with vim-like navigation
pub struct ChannelSelector {
    table: Table,
    channel_names: Vec<String>,
    max_col_size: usize,
}

/// Result of channel selection
#[derive(Debug, PartialEq)]
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
            max_col_size,
        }
    }

    /// Check if channel selection is needed
    pub fn needs_selection(&self, channel: &str) -> bool {
        channel.trim().is_empty() || !self.channel_names.contains(&channel.to_string())
    }

    /// Run the interactive channel selection.
    /// `/` starts a name filter; while it is open, letters narrow the list and arrows move.
    /// Returns SelectionResult::Selected(channel_name) if selected, SelectionResult::Cancelled if cancelled
    pub fn run<R: Read, W: Write>(&self, stdin: R, stdout: &mut W) -> Result<SelectionResult> {
        if self.channel_names.is_empty() {
            bail!("No channels to select");
        }

        let max_col_size = self.max_col_size;
        let mut filtering = false;
        let mut query = String::new();
        let mut names = self.channel_names.clone();
        let mut grid = selection_grid(&names, max_col_size)?;

        self.draw_names(stdout, &names, grid.as_ref(), filtering, &query);

        for key in stdin.keys() {
            match key? {
                Key::Ctrl('c') => return Ok(SelectionResult::Cancelled),
                Key::Esc if filtering => {
                    filtering = false;
                    query.clear();
                    names = self.channel_names.clone();
                    grid = selection_grid(&names, max_col_size)?;
                }
                Key::Char('q') if !filtering => return Ok(SelectionResult::Cancelled),
                Key::Char('/') if !filtering => {
                    filtering = true;
                }
                Key::Char('\n') => {
                    if let Some(grid) = grid.as_ref() {
                        return Ok(SelectionResult::Selected(selected_name(grid)));
                    }
                }
                Key::Backspace if filtering => {
                    query.pop();
                    if query.is_empty() {
                        filtering = false;
                    }
                    names = filter_names(&self.channel_names, &query);
                    grid = selection_grid(&names, max_col_size)?;
                }
                Key::Char(c) if filtering => {
                    query.push(c);
                    names = filter_names(&self.channel_names, &query);
                    grid = selection_grid(&names, max_col_size)?;
                }
                Key::Left | Key::Char('h') if !filtering => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.left();
                    }
                }
                Key::Right | Key::Char('l') if !filtering => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.right();
                    }
                }
                Key::Up | Key::Char('k') if !filtering => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.up();
                    }
                }
                Key::Down | Key::Char('j') if !filtering => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.down();
                    }
                }
                Key::Left => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.left();
                    }
                }
                Key::Right => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.right();
                    }
                }
                Key::Up => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.up();
                    }
                }
                Key::Down => {
                    if let Some(grid) = grid.as_mut() {
                        grid.table.cursor.down();
                    }
                }
                _ => {}
            }

            self.draw_names(stdout, &names, grid.as_ref(), filtering, &query);
        }

        match grid.as_ref() {
            Some(grid) => Ok(SelectionResult::Selected(selected_name(grid))),
            None => Ok(SelectionResult::Cancelled),
        }
    }

    /// Draw the table with the given channel selected
    pub fn draw<W: Write>(&self, stdout: &mut W, selected: &str) {
        self.table.draw(stdout, selected);
    }

    fn draw_names<W: Write>(
        &self,
        stdout: &mut W,
        names: &[String],
        grid: Option<&SelectionGrid>,
        filtering: bool,
        query: &str,
    ) {
        if let Some(grid) = grid.filter(|_| !names.is_empty()) {
            let table = Table::new("CHANNELS".to_string(), names.to_vec(), self.max_col_size);
            let selected = selected_name(grid);
            table.draw(stdout, &selected);
        } else {
            write!(
                stdout,
                "{}{}No matching channels\r\n",
                termion::cursor::Goto(1, 1),
                termion::clear::All
            )
            .unwrap();
        }
        if filtering {
            write!(stdout, "\r\n/{query}  (arrows move, Esc clears)").unwrap();
        }
        stdout.flush().unwrap();
    }
}

/// Channels whose name contains `query`, case-insensitively. An empty query keeps every name.
fn filter_names(names: &[String], query: &str) -> Vec<String> {
    if query.is_empty() {
        return names.to_vec();
    }
    let query = query.to_lowercase();
    names
        .iter()
        .filter(|name| name.to_lowercase().contains(&query))
        .cloned()
        .collect()
}

/// Cursor and the rows it indexes, captured together so a later terminal resize cannot
/// make the coordinates point at a different layout.
struct SelectionGrid {
    table: RposTable,
    rows: Vec<Vec<String>>,
}

fn selection_grid(names: &[String], max_col_size: usize) -> Result<Option<SelectionGrid>> {
    if names.is_empty() {
        return Ok(None);
    }
    let rows = Table::new("CHANNELS".to_string(), names.to_vec(), max_col_size).chunked_data();
    let widths = rows.iter().map(|row| row.len()).collect();
    Ok(Some(SelectionGrid {
        table: RposTable::new_jagged(widths)?
            .wrap_mode(WrapMode::Wrap)
            .vertical_policy(VerticalPolicy::SkipMissing),
        rows,
    }))
}

fn selected_name(grid: &SelectionGrid) -> String {
    let (row, col) = grid.table.cursor.current();
    grid.rows[row][col].clone()
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
    fn filter_names_is_case_insensitive() {
        let names = ["general", "random", "engineering"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            filter_names(&names, "EN"),
            ["general".to_string(), "engineering".to_string()]
        );
        assert_eq!(filter_names(&names, ""), names);
        assert!(filter_names(&names, "zzz").is_empty());
    }

    #[test]
    fn run_filters_then_selects() {
        let names = ["alpha", "help", "random"].map(String::from).to_vec();
        let selector = ChannelSelector::new(names, 10);
        let input = std::io::Cursor::new(b"/he\n");
        let mut output = Vec::new();
        let result = selector.run(input, &mut output).unwrap();
        assert_eq!(result, SelectionResult::Selected("help".to_string()));
    }

    #[test]
    fn run_cancel_with_q() {
        let selector = ChannelSelector::new(vec!["general".to_string()], 10);
        let input = std::io::Cursor::new(b"q");
        let mut output = Vec::new();
        let result = selector.run(input, &mut output).unwrap();
        assert_eq!(result, SelectionResult::Cancelled);
    }

    #[test]
    fn run_arrow_moves_while_filtering() {
        let names = ["help", "ops-help"].map(String::from).to_vec();
        // One column, so Down from the first row lands on the second match
        let selector = ChannelSelector::new(names, u16::MAX as usize);
        let input = std::io::Cursor::new(b"/help\x1b[B\n");
        let mut output = Vec::new();
        let result = selector.run(input, &mut output).unwrap();
        assert_eq!(result, SelectionResult::Selected("ops-help".to_string()));
    }
}
