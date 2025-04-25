#!/usr/bin/env bash

# Note: This script undoes what `post-sync` does, i.e, it removes the
# specific configuration so it doesn't appear in the repo. Read the
# post-sync example first for context.

# Portable version of `sed`.
sed_inplace() {
    sed -i.bak "$@" && rm "${@: -1}.bak"
}

# Unset Git email address.
#
# Maybe you don't want to expose your email address online, or you just
# like to keep things tidy and generic in the repo. In any case, this
# sets Git's email address to `<>` in the root's `.gitconfig` file.
[[ -n $DEEZ_VERBOSE ]] && echo "Unset Git email address."
git config --file ./.gitconfig user.email '<>'

# Remove local fish config.
#
# `sync` adds local fish configuration to fish's config file. This
# removes that config by truncating the file at the `# <deez>` comment
# (which we purposefully added just to make this operation easy).
[[ -n $DEEZ_VERBOSE ]] && echo "Remove local fish config."
sed_inplace '/^# <deez>$/,$d' ./.config/fish/config.fish
sed_inplace '${/^$/d;}' ./.config/fish/config.fish
