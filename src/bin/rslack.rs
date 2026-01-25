use std::io::{stdin, stdout};

use anyhow::{anyhow, Result};
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

use rslack::config::{Config, SLACK_URL};
use rslack::console::{print_messages, ChannelSelector, Editor, EditorResult, SelectionResult};
use rslack::option::{Opt, DEFAULT_MESSAGE_LIMIT};
use rslack::slack;

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
    let read_mode = opts.read;
    let limit = opts.limit.unwrap_or(DEFAULT_MESSAGE_LIMIT);

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

    // Get channel ID for API calls that require it
    let channel_info = channels
        .find_by_name(&channel)
        .ok_or_else(|| anyhow!("Channel '{}' not found", channel))?;

    // Read mode: fetch and display messages
    if read_mode {
        drop(stdout);

        let messages = slack_client
            .get_messages(&channel_info.id, limit)
            .await?;

        print_messages(&channel, &messages.messages);
        return Ok(());
    }

    // Write mode: compose and post message
    if Editor::needs_input(&message) {
        let mut editor = Editor::new();
        match editor.run(stdin(), &mut stdout, &channel)? {
            EditorResult::Submitted(msg) => message = msg,
            EditorResult::Cancelled => return Ok(()),
        }
    }

    drop(stdout);

    slack_client.post_message(&channel, &message).await?;
    println!("[Success] #{}\n {}", channel, message);

    Ok(())
}
