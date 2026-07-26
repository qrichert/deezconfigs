#!/usr/bin/env sh
#
# Mock `git` for `diff --incoming`, with an unreachable remote.

THIS_SCRIPTS_PARENT_DIR=$(dirname "$0")

echo "$@" >> $THIS_SCRIPTS_PARENT_DIR/output_git_args.txt

case "$1" in
fetch)
    echo "fatal: unable to access 'https://example.com/': Could not resolve host" >&2
    exit 128
    ;;
*)
    exit 1
    ;;
esac
