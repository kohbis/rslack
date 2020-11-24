use libc::{ioctl, winsize, TIOCGWINSZ, STDOUT_FILENO};
use std::io::{stdout, Result, Write};
use std::{mem, fs::File, os::unix::io::IntoRawFd};

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

fn row_string(size: usize) -> String {
    HYPHEN.repeat(size)
}

fn print_head_channels(size: usize) {
    let margin = size - HEAD.len();
    let margin_left = margin / 2;
    let margin_right = if margin % 2 == 0 { margin_left } else { margin_left + 1 };

    println!("|{}|", row_string(size));
    println!("|{}{}{}|", &WHITESPACE.repeat(margin_left), HEAD, &WHITESPACE.repeat(margin_right));
    println!("|{}|", row_string(size));
}

pub fn prompt(s: &str) -> Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(s.as_bytes())?;
    stdout.flush()
}

pub fn print_as_table(v: &[&str]) {
    let max_len = v.iter().max_by_key(|name| name.len()).unwrap().len() + 1;
    let ws_width = match terminal_size() {
        Some(ws) => ws.ws_col,
        None => 100
    };
    let col = ws_width as usize / (max_len + 2);

    let rows = v
                .chunks(col)
                .map(|chunk| {
                    chunk.iter()
                        .map(|cell| {
                            cell.to_string() + &WHITESPACE.repeat(max_len - cell.len())
                        })
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .collect::<Vec<_>>();

    print_head_channels(rows[0].len());

    for row in rows {
        println!("|{}|", row);
        println!("|{}|", row_string(row.len()))
    }
}
