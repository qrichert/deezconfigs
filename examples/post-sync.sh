#!/usr/bin/env bash

# Source machine-specific values (home, work, server, etc.).
#
# `~/.deezenv` is an arbitrary name, and may or may not exist. It
# contains environment variables that will get sourced and will override
# defaults to tailor the configs to the current session or machine.
[[ -f ~/.deezenv ]] && . ~/.deezenv

# Note: We don't bother here, but you could replace all occurrences of
# `~/` and `./` with `$DEEZ_HOME` and `$DEEZ_ROOT` respectively. This
# would enable using a custom Home (`HOME=... deez sync`), but it would
# also make the script more bloated.

# Note: All the steps here make use of the `$DEEZ_VERBOSE` environment
# variable. This is optional, but it enhances the user-experience as it
# shows exactly what is being done if the hook is run verbose mode. In
# normal mode, `$DEEZ_VERBOSE` will not be set and so the hooks will be
# silent.

# Set global Git email address.
#
# You likely use a different default email address at home than at work.
# This sets Git's email address to the value found in `~/.deezenv` or
# falls back to a default one.
[[ -n $DEEZ_VERBOSE ]] && echo "Set global Git email address."
git config --global user.email ${EMAIL:-you@example.com}

# Add local fish config.
#
# Different environments often require different shell configuration.
# The idea here is to keep the common config in the configs repo, and
# any specific config in a `~/.deezfish.fish` file.
#
# On `sync`, we append the specific config to the generic one, starting
# with a `# <deez>` comment, which will make removal easy for `rsync`.
#
# Another option here would have been to keep specific config files in
# the repo, ignore them with an `.ignore` file, and conditionnaly select
# the one to append based on `$DEEZ_OS` for instance.
if [[ -f ~/.deezfish.fish ]]; then
    [[ -n $DEEZ_VERBOSE ]] && echo "Add local fish config."
    echo -e "\n# <deez>\n" >> ~/.config/fish/config.fish
    cat ~/.deezfish.fish >> ~/.config/fish/config.fish
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
# This is a one-time configuration, so there won't be an undo in the
# `post-rsync` hook.
if ! grep -qF "SetEnv TERM=xterm-256color" ~/.ssh/config; then
    [[ -n $DEEZ_VERBOSE ]] && echo "Alias SSH terminfo for Ghostty."
    echo "" >> ~/.ssh/config
    cat ./.config/ghostty/ssh.txt >> ~/.ssh/config
fi
