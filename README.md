# rslack

[![Crates.io](https://img.shields.io/crates/v/spotter.svg)](https://crates.io/crates/rslack)
[![Documentation](https://docs.rs/rslack/badge.svg)](https://docs.rs/rslack)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

![select channel](https://github.com/kohbis/rslack/blob/main/docs/image/select_channel.png?raw=true)

## Features

- Interactive channel selection with vim-like navigation (h,j,k,l or arrow keys)
- Multi-line message editor with cursor movement
- Command-line options for direct channel and message specification
- Simple configuration via environment variables or config file

## Installation

### Using Homebrew

```bash
brew install kohbis/rslack/rslack
```

### From Source

```bash
git clone https://github.com/kohbis/rslack.git
cd rslack
cargo build --release
# The binary will be available at ./target/release/rslack
```

## Setup

### 1. Create a Slack App

1. Go to [Slack API: Create an App](https://api.slack.com/apps) and create a new app
2. Add your app to your workspace

### 2. Configure OAuth Permissions

1. Navigate to **OAuth & Permissions** in your app settings
2. Under **Scopes**, add the following **User Token Scopes**:
   - `channels:read` - To list available channels
   - `chat:write` - To post messages to channels
3. Click **Install App to Workspace**
4. Copy the **OAuth Access Token** from the **OAuth Tokens & Redirect URLs** section

### 3. Configure rslack

You can provide your Slack token in one of two ways:

#### Option 1: Environment Variable

```bash
export RSLACK_TOKEN=xoxp-your-token-here
```

#### Option 2: Configuration File

Create a `.rslack` file in your home directory:

```bash
echo "RSLACK_TOKEN=xoxp-your-token-here" > ${HOME}/.rslack
```

**Note:** If both methods are used, the configuration file takes precedence.

## Usage

### Basic Usage

Simply run the command to start the interactive interface:

```bash
rslack
```

This will:
1. Display a list of available channels
2. Allow you to select a channel using navigation keys
3. Open a message editor where you can type your message
4. Post the message to the selected channel

### Navigation Keys

- Channel selection: Arrow keys or vim-style `h`, `j`, `k`, `l`
- Confirm selection: `Enter`
- Exit: `q` or `Ctrl+c`

### Message Editor

- Type your message (supports multi-line messages)
- Move cursor: Up/Down arrow keys
- Post message: `Ctrl+p`
- Exit without posting: `Ctrl+c`

### Command-line Options

You can bypass the interactive interface by specifying options:

```bash
# Post to a specific channel
rslack -c general

# Post a specific message to a specific channel
rslack -c general -m "Hello, world!"
```

Options:
- `-c, --channel <CHANNEL>`: Specify the channel to post to
- `-m, --message <MESSAGE>`: Specify the message to post

## Development

### Building and Running Locally

```bash
# Build and run in release mode
cargo run --release

# Build only
cargo build --release
```

### Running Tests

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
