# rslack

![select channel](https://github.com/kohbis/rslack/blob/main/docs/image/select_channel.png?raw=true)

## Setting

1. [Create new app](https://api.slack.com/apps)

1. Create OAuth Token
    **OAuth & Permissions**
    1. **User Token Scopes**
        - channels:read
        - chat:write
    1. **OAuth Tokens & Redirect URLs**
        1. **Install App to Workspace**
        2. **Tokens for Your Workspace** -> copy **OAuth Access Token**

1. Setting OAuth Token

    ```bash
    export RSLACK_TOKEN=your-token
    ```

## Usage

```bash
# Install
brew install kohbis/rslack/rslack

# Configuration
# If both are set, use the value of `.rslack
export RSLACK_TOKEN=your-token
# or
echo "RSLACK_TOKEN=your-token" > ${HOME}/.rslack

rslack
```

### Local Build & Run

```bash
# Configuration
# and
cargo run --release
```

## Test

```bash
cargo test
```
