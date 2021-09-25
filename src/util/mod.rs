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
    fn argument_with_default() {
        let channels = vec!["apple", "grape", "orange"];
        assert_eq!(6, max_channel_size(&channels),)
    }
}
