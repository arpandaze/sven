if ( ! $?SECRETS_LOADED ) then
    setenv SECRETS_LOADED 1
    eval `sven export --shell csh`
endif

alias precmd 'if ( ! $?SECRETS_LOADED ) eval `sven export --shell csh`'
