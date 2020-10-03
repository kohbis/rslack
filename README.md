# rslack

## Setting

1. [Create new app](https://api.slack.com/apps)

1. Create OAuth Token
    **OAuth & Permissions**
    1. **User Token Scopes** -> add **channels:read**
    1. **OAuth Tokens & Redirect URLs**
        1. **Install App to Workspace**
        2. **Tokens for Your Workspace** -> copy **OAuth Access Token**

1. Setting `.token`

    ```bash
    $ mv .token.yours .token
    # overwrite the token
    ```

## Usage

```bash
$ cargo run
```
