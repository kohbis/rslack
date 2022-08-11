# rslack

![select channel](https://github.com/kohbis/rslack/blob/main/doc/image/select_channel.png?raw=true)

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

    # or

    mv .token.keep .token
    ```

    If both are set, use the value of `.token`

## Usage

```bash
brew install kohbis/rslack/rslack
```

If you want by local build.

```bash
cargo run
```

If you want to run on Docker, exec the following command.

```bash
docker build -t rslack -f docker/Dockerfile .
docker run --rm -e RSLACK_TOKEN=your-token -ti rslack
```

## Test

```bash
cargo test
```
