use std::io::{stdin, stdout};
use std::process::ExitCode;

use anyhow::{anyhow, bail, Result};
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

use rslack::config::{Config, SLACK_URL};
use rslack::console::{
    channel_heading, print_message_table, print_messages, ChannelSelector, Editor, EditorResult,
    SelectionResult,
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
    let thread = match opts.thread {
        Some(ts) => {
            let ts = ts.trim();
            if ts.is_empty() {
                bail!("Thread timestamp is empty");
            }
            Some(ts.to_string())
        }
        None => None,
    };

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
    let mut channels = slack_client.get_channels().await?;
    if !channels.unavailable.is_empty() {
        let kinds = channels
            .unavailable
            .iter()
            .map(|kind| slack::conversation_type_label(kind))
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("[Warning] Not listing {kinds}: the token is missing the scope");
    }
    if let Err(err) = slack_client
        .resolve_direct_message_names(&mut channels.channels)
        .await
    {
        let hint = if slack::SlackApiError::is(&err, "missing_scope") {
            " (add the users:read scope to your Slack app and reinstall it)"
        } else {
            ""
        };
        eprintln!(
            "[Warning] Showing user IDs for direct messages; failed to get user names: {}{}",
            err, hint
        );
    }
    channels.disambiguate_labels();

    let channel_names = channels.channel_names();
    let max_col_size = channels.max_channel_size() + 1;
    let selector = ChannelSelector::new(channel_names.clone(), max_col_size);

    // An id or `@name` counts even when it is not the label drawn in the selector
    let needs_selection = channel.trim().is_empty() || channels.find_by_name(&channel).is_none();
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
            let title = match &thread {
                Some(ts) => format!("{channel} (thread {ts})"),
                None => channel.clone(),
            };
            match editor.run(stdin(), &mut stdout, &title)? {
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
        let mut messages = match &thread {
            Some(ts) => {
                slack_client
                    .get_replies(&channel_info.id, ts, limit)
                    .await?
            }
            None => slack_client.get_messages(&channel_info.id, limit).await?,
        };
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
            MessageFormat::Text => print_messages(&channel, thread.as_deref(), &messages.messages),
            MessageFormat::Table => {
                print_message_table(&channel, thread.as_deref(), &messages.messages)
            }
        }
        return Ok(());
    }

    // Write mode: post the message or upload the image
    if let Some(path) = &image {
        slack_client
            .upload_file(&channel_info.id, path, Some(&message), thread.as_deref())
            .await?;
        println!(
            "[Success] {}\n {}\n {}",
            channel_heading(&channel),
            path.display(),
            message
        );
        if let Some(ts) = &thread {
            println!(" thread: {ts}");
        }
        return Ok(());
    }

    let posted = slack_client
        .post_message(&channel_info.id, &message, thread.as_deref())
        .await?;
    println!("[Success] {}\n {}", channel_heading(&channel), message);
    if let Some(ts) = &posted.ts {
        println!(" ts: {ts}");
    }
    if let Some(ts) = &thread {
        println!(" thread: {ts}");
    }

    Ok(())
}
