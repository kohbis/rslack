use crate::api::SlackChannel;

pub fn slack_channel_names(channels: &Vec<SlackChannel>) -> Vec<&str> {
    channels
        .iter()
        .map(|channel| channel.name.as_str())
        .collect()
}

pub fn chunk_slack_channel_names<'a>(
    chunk_size: usize,
    channel_names: &Vec<&'a str>,
) -> Vec<Vec<&'a str>> {
    channel_names
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

pub fn max_channel_size(channel_names: &Vec<&str>) -> usize {
    match channel_names.iter().max_by_key(|name| name.len()) {
        Some(name) => name.len(),
        _ => 80,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_build_slack_channel_names() {
        let channels = vec!["apple", "grape", "orange"];
        let slack_channels: Vec<SlackChannel> = channels
            .iter()
            .map(|channel| SlackChannel {
                name: channel.to_string(),
            })
            .collect();
        assert_eq!(channels, slack_channel_names(&slack_channels));
    }

    #[test]
    fn it_chunks_slack_channel_names() {
        let channel_names = vec!["apple", "grape", "orange", "peach"];
        let expected = vec![vec!["apple", "grape", "orange"], vec!["peach"]];
        assert_eq!(expected, chunk_slack_channel_names(3, &channel_names));
    }

    #[test]
    fn it_get_max_channel_size() {
        let channels = vec!["apple", "grape", "orange"];
        assert_eq!(6, max_channel_size(&channels),)
    }
}
