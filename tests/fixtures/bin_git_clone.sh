#!/usr/bin/env sh

THIS_SCRIPTS_PARENT_DIR=$(dirname "$0")

for arg do
    case "$arg" in
        */deez-*) clone_path=$arg ;;
    esac
done

mkdir -p "$clone_path"
touch "$clone_path/.deez"
touch "$clone_path/.gitconfig"
mkdir -p "$clone_path/sub/root"
touch "$clone_path/sub/root/.deez"
touch "$clone_path/sub/root/.gitconfig"
echo "$clone_path" > "$THIS_SCRIPTS_PARENT_DIR/output_clone_path.txt"

case "$*" in
    *fail*) exit 42 ;;
esac
