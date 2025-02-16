default:
    @just --list

# Install sven binary and shell integration
install: build install-binary install-shell

# Build the release binary
build:
    cargo build --release

# Install the binary to ~/.local/bin
install-binary:
    mkdir -p ~/.local/bin
    cp target/release/sven ~/.local/bin/
    chmod +x ~/.local/bin/sven

# Install shell integration based on current shell
install-shell: detect-shell
    #!/usr/bin/env sh
    case "${SHELL}" in
        */fish)
            mkdir -p ~/.config/fish/functions
            cp shell/load_secrets.fish ~/.config/fish/functions/
            echo "Installed fish integration"
            ;;
        */bash)
            cp shell/load_secrets.bash ~/.bashrc.d/sven.bash 2>/dev/null || \
            echo ". ${HOME}/.local/share/sven/load_secrets.bash" >> ~/.bashrc
            mkdir -p ~/.local/share/sven
            cp shell/load_secrets.bash ~/.local/share/sven/
            echo "Installed bash integration"
            ;;
        */zsh)
            cp shell/load_secrets.zsh ~/.zshrc.d/sven.zsh 2>/dev/null || \
            echo ". ${HOME}/.local/share/sven/load_secrets.zsh" >> ~/.zshrc
            mkdir -p ~/.local/share/sven
            cp shell/load_secrets.zsh ~/.local/share/sven/
            echo "Installed zsh integration"
            ;;
        */csh|*/tcsh)
            echo "source ${HOME}/.local/share/sven/load_secrets.csh" >> ~/.cshrc
            mkdir -p ~/.local/share/sven
            cp shell/load_secrets.csh ~/.local/share/sven/
            echo "Installed csh/tcsh integration"
            ;;
        *)
            echo "Unsupported shell: ${SHELL}"
            echo "Please manually source one of the following files:"
            echo "  - shell/load_secrets.bash (for bash)"
            echo "  - shell/load_secrets.zsh  (for zsh)"
            echo "  - shell/load_secrets.fish (for fish)"
            echo "  - shell/load_secrets.csh  (for csh/tcsh)"
            exit 1
            ;;
    esac

# Detect current shell
detect-shell:
    #!/usr/bin/env sh
    if [ -z "${SHELL}" ]; then
        echo "SHELL environment variable not set"
        exit 1
    fi

# Uninstall sven and all shell integrations
uninstall:
    rm -f ~/.local/bin/sven
    rm -f ~/.config/fish/functions/load_secrets.fish
    rm -f ~/.local/share/sven/load_secrets.*
    rm -rf ~/.local/share/sven
    rm -rf ~/.config/sven

# Clean build artifacts
clean:
    cargo clean