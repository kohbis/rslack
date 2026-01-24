use std::io::{stdin, stdout};

use anyhow::Result;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

use rslack::config::Config;
use rslack::console::{ChannelSelector, Editor, SelectionResult};
use rslack::option::Opt;
use rslack::slack;

const SLACK_URL: &str = "https://slack.com";

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{}", err);
    }
}

async fn run() -> Result<()> {
    let opts = Opt::get_opts();
    let mut channel = opts.channel.unwrap_or_default();
    let mut message = opts.message.unwrap_or_default();

    let config = Config::new(None)?;
    let slack_client = slack::SlackClient::new(&config, SLACK_URL);
    let channels = slack_client.get_channels().await?;

    let channel_names = channels.channel_names();
    let max_col_size = channels.max_channel_size() + 1;
    let selector = ChannelSelector::new(channel_names.clone(), max_col_size);

    let stdout = stdout().into_raw_mode()?;
    let mut stdout = stdout.into_alternate_screen()?;

    if selector.needs_selection(&channel) {
        match selector.run(stdin(), &mut stdout)? {
            SelectionResult::Selected(selected) => channel = selected,
            SelectionResult::Cancelled => return Ok(()),
        }
    }
    selector.draw(&mut stdout, &channel);

    if message.trim().is_empty() {
        let mut editor = Editor::new();
        editor.draw_header(&mut stdout, &channel);

        let stdin = stdin();
        for c in stdin.keys() {
            match c? {
                Key::Ctrl('c') => return Ok(()),
                Key::Ctrl('p') => {
                    if message.trim().is_empty() {
                        editor.clear(&mut stdout);
                        continue;
                    } else {
                        break;
                    }
                }
                Key::Char('\n') => {
                    editor.new_line();
                }
                Key::Char(c) => {
                    editor.insert(c);
                }
                Key::Up => {
                    editor.cursor_up();
                }
                Key::Down => {
                    editor.cursor_down();
                }
                Key::Backspace => {
                    editor.backspace(&mut stdout);
                }
                _ => {}
            }

            editor.draw_message(&mut stdout);
            message = editor.message();
        }
    }

    drop(stdout);

    slack_client.post_message(&channel, &message).await?;
    println!("[Success] #{}\n {}", channel, message);

    Ok(())
}
