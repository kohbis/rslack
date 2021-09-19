use std::io::{stdout, Result, Write};

use termion::color;
use termion::raw::IntoRawMode;
use termion::terminal_size;

const BAR: &str = "|";
const WHITESPACE: &str = " ";
const HYPHEN: &str = "-";
const HEAD: &str = "CHANNELS";

fn print_row(stdout: &mut dyn Write, content: &str) {
    write!(stdout, "{}{}{}", BAR, content, BAR).unwrap();
    write!(stdout, "\r\n").unwrap();
}

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

pub fn prompt(s: &str) -> Result<()> {
    let mut stdout = stdout().into_raw_mode().unwrap();

    write!(stdout, "{}", s)?;
    stdout.flush()
}

pub fn print_as_table(channels: &[&str], selected: &str) {
    let mut stdout = stdout().into_raw_mode().unwrap();

    write!(
        stdout,
        "{}{}",
        termion::cursor::Goto(1, 1),
        termion::clear::All
    )
    .unwrap();

    let max_len = channels.iter().max_by_key(|name| name.len()).unwrap().len() + 1;
    let ws_width = if let Ok((width, _height)) = terminal_size() {
        width
    } else {
        100
    };
    let col = ws_width as usize / (max_len + 2);

    let rows = channels
        .chunks(col)
        .map(|chunk| {
            (
                chunk.len() * (max_len + 2) - 1,
                chunk
                    .into_iter()
                    .map(|&cell| {
                        let (fg_color, bg_color) = if cell == selected {
                            (
                                color::Fg(color::Black).to_string(),
                                color::Bg(color::Blue).to_string(),
                            )
                        } else {
                            (
                                color::Fg(color::Reset).to_string(),
                                color::Bg(color::Reset).to_string(),
                            )
                        };

                        format!(
                            "{}{}{}{}{}{}{}",
                            WHITESPACE,
                            bg_color,
                            fg_color,
                            cell.to_string(),
                            color::Fg(color::Reset),
                            color::Bg(color::Reset),
                            &WHITESPACE.repeat(max_len - cell.len())
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<(usize, Vec<String>)>>();

    print_head_channels(&mut stdout, rows[0].0);

    for row in rows {
        print_row(&mut stdout, &row.1.join(BAR));
        print_row(&mut stdout, &horizontal_rule(row.0));
    }
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
