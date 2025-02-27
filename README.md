# Sven - Secure Environment Variables Manager

Sven is a secure environment variable manager. It encrypts environment variable at rest using GPG and automatically injects it to shell.

## Prerequisites

- Rust toolchain
- GPG setup with at least one key with ultimate trust
- just (optional, for easy installation)

## Installation

Using just:
```bash
just install
```

Manual installation:
```bash
# Build
cargo build --release

# Install binary
mkdir -p ~/.local/bin
cp target/release/sven ~/.local/bin/
chmod +x ~/.local/bin/sven

# Append to fish config
sven export --shell fish | source
```

## Usage

Add a secret:
```bash
sven add GITHUB_TOKEN "your-token-here"
```

List all secret keys:
```bash
sven list
```

Remove a secret:
```bash
sven remove GITHUB_TOKEN
```

Export secrets to shell:
```bash
sven export
```

### Daemon Mode

Sven now supports a daemon mode that keeps decrypted secrets in memory, which significantly improves performance when using secrets across multiple shells or commands.

Start the daemon (unlock secrets and keep them in memory):
```bash
sven unlock
```

Check daemon status:
```bash
sven status
```

Stop the daemon:
```bash
sven stop
```

When the daemon is running, all commands (add, remove, list, export) will automatically use it, avoiding the need to decrypt secrets each time.

## Uninstallation

Using just:
```bash
just uninstall
```

Manual uninstallation:
```bash
rm -f ~/.local/bin/sven
rm -f ~/.config/fish/functions/load_secrets.fish
rm -rf ~/.config/sven
```
