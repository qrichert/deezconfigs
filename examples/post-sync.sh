#!/usr/bin/env bash

# Source machine-specific values (home, work, server, etc.).
#
# `~/.deezenv` is an arbitrary name, and may or may not exist. It
# contains environment variables that will get sourced and will override
# defaults to tailor the configs to the current session or machine.
[[ -f ~/.deezenv ]] && . ~/.deezenv

# Note: We don't bother here, but you could replace all occurrences of
# `~/` and `./` with `$DEEZ_HOME` and `$DEEZ_ROOT` respectively. This
# would enable using a custom home (`HOME=... deez sync`), but it would
# also make the script more bloated.

# Note: All the steps here make use of the `$DEEZ_VERBOSE` environment
# variable. This is optional, but it enhances the user-experience as it
# shows exactly what is being done if the hook is run verbose mode. In
# normal mode, `$DEEZ_VERBOSE` will not be set and so the hooks will be
# silent.

# Create local Git config if missing.
#
# You likely use a different email address at home than at work. The
# managed `.gitconfig` leaves `user.email` out and instead ends with:
#
#     [include]
#         path = ~/.gitconfig.local
#
# This creates that local file with the email address found in
# `~/.deezenv`, or a default one. Existing files are left untouched, so
# you can edit them freely, and the managed `.gitconfig` stays in sync
# with the repo (no `post-rsync` cleanup needed).
if [[ ! -e ~/.gitconfig.local ]]; then
    [[ -n $DEEZ_VERBOSE ]] && echo "Create local Git config."
    git config --file ~/.gitconfig.local user.email "${EMAIL:-you@example.com}"
fi

# Create local fish config if missing.
#
# Different environments often require different shell configuration.
# The common config lives in the repo, and ends with:
#
#     if test -f "$HOME/.local.fish"
#         source "$HOME/.local.fish"
#     end
#
# Machine-specific config goes into `~/.local.fish`. As with Git, this
# only creates the file (empty) if missing, and never touches the
# managed `config.fish`.
if [[ ! -e ~/.local.fish ]]; then
    [[ -n $DEEZ_VERBOSE ]] && echo "Create local fish config."
    touch ~/.local.fish
fi

# Trim Neovim config on low-powered machines.
#
# Installing, updating, and running plugins and LSPs doesn't make a dent
# in regular desktop machines, but it can be a real bottleneck on
# low-powered VPSs, where a full-blown editor is rarely even useful.
#
# Here we're detecting such machines (< 2 CPUs), and trimming Neovim's
# config file accordingly at the `-- END OF MINIMAL CONFIG --` marker.
# This marker has been placed strategically in the config file to
# delimit regular Neovim configuration from plugins and LSPs.
nb_cpu_cores=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 0)
if (( $nb_cpu_cores < 2 )); then
    [[ -n $DEEZ_VERBOSE ]] && echo "Low-powered machine: trimming Neovim \`init.lua\` to minimal config."
    sed '/-- END OF MINIMAL CONFIG --/q' ~/.config/nvim/init.lua > /tmp/init.lua
    mv /tmp/init.lua ~/.config/nvim/init.lua
fi

# Alias SSH terminfo for Ghostty.
#
# At the time of writing, Ghostty is still quite new and is not
# recognized by many tools. This adds a rule to the SSH config to force
# full-color mode on every host.
#
# How it works is it looks for the rule in the SSH config, and if it
# can't find it, add the config. If it does find it, this is a no-op.
#
# This edits a file outside the repo, so there is nothing to undo on
# `rsync`.
if ! grep -qF "SetEnv TERM=xterm-256color" ~/.ssh/config; then
    [[ -n $DEEZ_VERBOSE ]] && echo "Alias SSH terminfo for Ghostty."
    echo "" >> ~/.ssh/config
    cat ./.config/ghostty/ssh.txt >> ~/.ssh/config
fi
