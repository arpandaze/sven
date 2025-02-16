__sven_load_secrets() {
    if [ -z "$SECRETS_LOADED" ]; then
        export SECRETS_LOADED=1
        eval "$(sven export --shell zsh)"
    fi
}

# Load secrets on shell start
__sven_load_secrets

# Add to chpwd hook for directory change detection
autoload -Uz add-zsh-hook
add-zsh-hook chpwd __sven_load_secrets
