function load_secrets --on-variable PWD
    if not set -q SECRETS_LOADED
        set -g SECRETS_LOADED 1
        eval (sven export --shell fish)
    end
end

# Initial load when shell starts
if not set -q SECRETS_LOADED
    set -g SECRETS_LOADED 1
    eval (sven export --shell fish)
end
