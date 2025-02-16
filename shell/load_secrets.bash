__sven_load_secrets() {
    if [ -z "$SECRETS_LOADED" ]; then
        export SECRETS_LOADED=1
        eval "$(sven export --shell bash)"
    fi
}

# Load secrets on shell start
__sven_load_secrets

# Add to PROMPT_COMMAND to check when directory changes
if [[ $PROMPT_COMMAND != *"__sven_load_secrets"* ]]; then
    PROMPT_COMMAND="__sven_load_secrets;$PROMPT_COMMAND"
fi
