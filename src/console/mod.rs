use std::io::Write;

use termion::color;
use termion::terminal_size;
use unicode_width::UnicodeWidthStr;

const BAR: &str = "|";
const WHITESPACE: &str = " ";
const HYPHEN: &str = "-";
const HEAD: &str = "CHANNELS";
const USAGE_CHANNEL_MEASSAGE: &str = "Select by ← ↓ ↑ → or h j k l, and Enter.";

/*
 * Print table row with bar.
 */
fn print_row(stdout: &mut dyn Write, content: &str) {
    write!(stdout, "{}{}{}{}", BAR, content, BAR, "\r\n").unwrap();
}

/*
 * Print table header.
 */
fn print_head_channels(stdout: &mut dyn Write, size: usize) {
    let margin = size - HEAD.len();
    let margin_left = margin / 2;
    let margin_right = if margin % 2 == 0 {
        margin_left
    } else {
        margin_left + 1
    };

    let horizontal_rule = horizontal_rule(size);
    let head = [
        &WHITESPACE.repeat(margin_left),
        HEAD,
        &WHITESPACE.repeat(margin_right),
    ]
    .concat();

    print_row(stdout, &horizontal_rule);
    print_row(stdout, &head);
    print_row(stdout, &horizontal_rule);
}

fn horizontal_rule(size: usize) -> String {
    HYPHEN.repeat(size)
}

/*
 * Get terminal window size.
 */
pub fn term_size() -> (u16, u16) {
    match terminal_size() {
        Ok((width, height)) => (width, height),
        _ => (100, 100),
    }
}

/*
 * Print channel names as table.
 */
pub fn print_as_table(
    stdout: &mut dyn Write,
    channel_names: &Vec<Vec<String>>,
    max_col_size: usize,
    selected: &str,
) {
    write!(
        stdout,
        "{}{}",
        termion::cursor::Goto(1, 1),
        termion::clear::All
    )
    .unwrap();

    let rows: Vec<(usize, Vec<String>)> = channel_names
        .iter()
        .map(|names| {
            (
                names.len() * (max_col_size + 2) - 1,
                names
                    .into_iter()
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
                            "{}{}{}{}{}{}{}",
                            bg_color,
                            WHITESPACE,
                            fg_color,
                            cell,
                            color::Fg(color::Reset),
                            &WHITESPACE
                                .repeat(max_col_size - UnicodeWidthStr::width(cell.as_str())),
                            color::Bg(color::Reset),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    // Print table of channel names
    print_head_channels(stdout, rows[0].0);
    for row in rows {
        print_row(stdout, &row.1.join(BAR));
        print_row(stdout, &horizontal_rule(row.0));
    }
    write!(stdout, "{}", USAGE_CHANNEL_MEASSAGE).unwrap();
    stdout.flush().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_string_between_bars() {
        let mut stdout = Vec::new();
        print_row(&mut stdout, "string");

        assert_eq!(stdout, b"|string|\r\n")
    }

    #[test]
    fn horizontal_rule_with_size() {
        let size = 5;
        let actual = horizontal_rule(size);
        let expected = "-----".to_string();
        assert_eq!(actual, expected)
    }
}
