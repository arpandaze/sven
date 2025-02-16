default:
    @just --list

# Install sven binary and fish integration
install: build install-binary install-fish

# Build the release binary
build:
    cargo build --release

# Install the binary to ~/.local/bin
install-binary:
    mkdir -p ~/.local/bin
    cp target/release/sven ~/.local/bin/
    chmod +x ~/.local/bin/sven

# Install fish integration
install-fish:
    mkdir -p ~/.config/fish/functions
    cp load_secrets.fish ~/.config/fish/functions/

# Uninstall sven and fish integration
uninstall:
    rm -f ~/.local/bin/sven
    rm -f ~/.config/fish/functions/load_secrets.fish
    rm -rf ~/.config/sven

# Clean build artifacts
clean:
    cargo clean