function load_secrets --on-variable PWD
    if test "$PWD" = "$HOME"
        # Only load secrets once when we enter the home directory
        if not set -q SECRETS_LOADED
            set -g SECRETS_LOADED 1
            eval (sven export)
        end
    end
end

# Initial load when shell starts
if test "$PWD" = "$HOME"
    if not set -q SECRETS_LOADED
        set -g SECRETS_LOADED 1
        eval (sven export)
    end
end