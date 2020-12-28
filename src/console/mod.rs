use libc::{ioctl, winsize, TIOCGWINSZ, STDOUT_FILENO};
use std::io::{stdout, Result, Write};
use std::{mem, fs::File, os::unix::io::IntoRawFd};

const BAR: &str = "|";
const WHITESPACE: &str = " ";
const HYPHEN: &str = "-";
const HEAD: &str = "CHANNELS";

fn terminal_size() -> Option<winsize> {
    let fd = if let Ok(file) = File::open("/dev/tty"){
        file.into_raw_fd()
    }else {
        STDOUT_FILENO
    };

    let mut ws: winsize = unsafe { mem::zeroed() };
    if unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) } == -1 {
        None
    } else {
        Some(ws)
    }
}

fn print_line(content: &str) {
    println!("{}{}{}", BAR, content, BAR)
}

fn print_head_channels(size: usize) {
    let margin = size - HEAD.len();
    let margin_left = margin / 2;
    let margin_right = if margin % 2 == 0 { margin_left } else { margin_left + 1 };

    let horizontal_rule = horizontal_rule(size);
    let head = [&WHITESPACE.repeat(margin_left), HEAD, &WHITESPACE.repeat(margin_right)].concat();

    print_line(&horizontal_rule);
    print_line(&head);
    print_line(&horizontal_rule);
}

fn horizontal_rule(size: usize) -> String {
    HYPHEN.repeat(size)
}

pub fn prompt(s: &str) -> Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(s.as_bytes())?;
    stdout.flush()
}

pub fn print_as_table(channels: &[&str]) {
    let max_len = channels.iter().max_by_key(|name| name.len()).unwrap().len() + 1;
    let ws_width = match terminal_size() {
        Some(ws) => ws.ws_col,
        None => 100
    };
    let col = ws_width as usize / (max_len + 2);

    let rows = channels
                .chunks(col)
                .map(|chunk| {
                    chunk.iter()
                        .map(|cell| {
                            cell.to_string() + &WHITESPACE.repeat(max_len - cell.len())
                        })
                        .collect::<Vec<_>>()
                        .join(BAR)
                })
                .collect::<Vec<_>>();

    print_head_channels(rows[0].len());

    for row in rows {
        print_line(&row);
        print_line(&horizontal_rule(row.len()));
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_rule_with_size() {
        let size = 5;
        let expected  = "-----".to_string();
        let actual = horizontal_rule(size);
        assert_eq!(expected, actual)
    }
}
