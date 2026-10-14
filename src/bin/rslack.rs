use std::io::{stdin, stdout};
use std::process::ExitCode;

use anyhow::{anyhow, bail, Result};
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

use rslack::config::{Config, SLACK_URL};
use rslack::console::{
    print_message_table, print_messages, ChannelSelector, Editor, EditorResult, SelectionResult,
};
use rslack::option::{read_message, MessageFormat, Opt, DEFAULT_MESSAGE_LIMIT};
use rslack::slack;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let opts = Opt::get_opts();
    let mut channel = opts.channel.unwrap_or_default();
    let mut message = opts.message.unwrap_or_default();
    let read_mode = opts.read;
    let limit = opts.limit.unwrap_or(DEFAULT_MESSAGE_LIMIT);
    let image = opts.image;
    let format = opts.format;

    if let Some(path) = &image {
        if !path.is_file() {
            return Err(anyhow!("{}: No such file", path.display()));
        }
    }

    // A piped stdin carries the message (e.g. `echo hi | rslack -c general`),
    // so it can't also be used for the interactive screens
    let interactive = termion::is_tty(&stdin());
    if !read_mode && message.is_empty() && !interactive {
        message = read_message(stdin().lock())?;
        // The comment is optional when uploading an image
        if image.is_none() && message.trim().is_empty() {
            bail!("No message to post: stdin is empty");
        }
    }

    let config = Config::new(None)?;
    let slack_client = slack::SlackClient::new(&config, SLACK_URL);
    let channels = slack_client.get_channels().await?;

    let channel_names = channels.channel_names();
    let max_col_size = channels.max_channel_size() + 1;
    let selector = ChannelSelector::new(channel_names.clone(), max_col_size);

    // Switch to the interactive screen only when something has to be selected or typed
    let needs_selection = selector.needs_selection(&channel);
    if needs_selection && !interactive {
        if channel.trim().is_empty() {
            bail!("Specify a channel with -c when stdin is not a terminal");
        }
        bail!("Channel '{}' not found", channel);
    }
    // The comment is optional when uploading an image
    let needs_editor = !read_mode && image.is_none() && Editor::needs_input(&message);
    if needs_selection || needs_editor {
        let stdout = stdout().into_raw_mode()?;
        let mut stdout = stdout.into_alternate_screen()?;

        if needs_selection {
            match selector.run(stdin(), &mut stdout)? {
                SelectionResult::Selected(selected) => channel = selected,
                SelectionResult::Cancelled => return Ok(()),
            }
        }

        if needs_editor {
            let mut editor = Editor::new();
            match editor.run(stdin(), &mut stdout, &channel)? {
                EditorResult::Submitted(msg) => message = msg,
                EditorResult::Cancelled => return Ok(()),
            }
        }
    }

    // Get channel ID for API calls that require it
    let channel_info = channels
        .find_by_name(&channel)
        .ok_or_else(|| anyhow!("Channel '{}' not found", channel))?;

    // Read mode: fetch and display messages
    if read_mode {
        let mut messages = slack_client.get_messages(&channel_info.id, limit).await?;
        if let Err(err) = slack_client
            .resolve_user_names(&mut messages.messages)
            .await
        {
            let hint = if slack::SlackApiError::is(&err, "missing_scope") {
                " (add the users:read scope to your Slack app and reinstall it)"
            } else {
                ""
            };
            eprintln!(
                "[Warning] Showing user IDs; failed to get user names: {}{}",
                err, hint
            );
        }

        match format {
            MessageFormat::Text => print_messages(&channel, &messages.messages),
            MessageFormat::Table => print_message_table(&channel, &messages.messages),
        }
        return Ok(());
    }

    // Write mode: post the message or upload the image
    if let Some(path) = &image {
        slack_client
            .upload_file(&channel_info.id, path, Some(&message))
            .await?;
        println!("[Success] #{}\n {}\n {}", channel, path.display(), message);
        return Ok(());
    }

    slack_client
        .post_message(&channel_info.id, &message)
        .await?;
    println!("[Success] #{}\n {}", channel, message);

    Ok(())
}
