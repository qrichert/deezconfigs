#!/usr/bin/env sh
#
# Mock `git` for `diff --incoming`, with no upstream branch configured.

THIS_SCRIPTS_PARENT_DIR=$(dirname "$0")

echo "$@" >> $THIS_SCRIPTS_PARENT_DIR/output_git_args.txt

case "$1" in
fetch)
    exit 0
    ;;
diff)
    echo "fatal: no upstream configured for branch 'main'" >&2
    exit 128
    ;;
*)
    exit 1
    ;;
esac
