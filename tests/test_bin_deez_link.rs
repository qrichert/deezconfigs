// deezconfigs — Manage deez config files.
// Copyright (C) 2025  Quentin Richert
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

mod conf;
mod run;

use std::fs;
use std::path::{Path, PathBuf};

use conf::{CONFIGS, HOME};
use run::{run, run_in_dir};

fn file_exists_in_home(file_path: &str) -> bool {
    let file = PathBuf::from(HOME).join(file_path);
    file.is_file()
}

fn symlink_exists_in_home(symlink_path: &str) -> bool {
    let symlink = PathBuf::from(HOME).join(symlink_path);
    symlink.is_symlink()
}

fn read(file_path: &Path) -> String {
    fs::read_to_string(file_path).unwrap()
}

fn read_in_home(file_path: &str) -> String {
    let file = PathBuf::from(HOME).join(file_path);
    fs::read_to_string(file).unwrap()
}

#[test]
fn link_regular() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(symlink_exists_in_home(".gitconfig"));
    assert!(symlink_exists_in_home(".config/nvim/init.lua"));
    assert!(symlink_exists_in_home(".config/fish/config.fish"));
    assert!(symlink_exists_in_home(".config/ghostty/config"));
}

#[test]
fn link_points_to_correct_file() {
    conf::init();

    conf::create_file_in_configs("foo.txt", Some("this is foo"));
    conf::create_file_in_configs("bar/baz.txt", Some("this is bar/baz"));

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read_in_home("foo.txt"), "this is foo");
    assert_eq!(read_in_home("bar/baz.txt"), "this is bar/baz");
}

#[test]
fn link_output() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);

    let output = run(&["link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // File order is non-deterministic.
    assert!(!output.stdout.contains("hook: pre-link.sh"));
    assert!(!output.stdout.contains(".gitconfig"));
    assert!(!output.stdout.contains(".config/nvim/init.lua"));
    assert!(!output.stdout.contains(".config/fish/config.fish"));
    assert!(!output.stdout.contains(".config/ghostty/config"));
    assert!(output.stdout.contains("Wrote 4 files."));
    assert!(!output.stdout.contains("hook: post-link.sh"));
    assert!(output.stdout.contains("Ran 2 hooks"));
}

#[test]
fn link_output_verbose() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // File order is non-deterministic.
    assert!(output.stdout.contains("hook: pre-link.sh"));
    assert!(output.stdout.contains(".gitconfig"));
    assert!(output.stdout.contains(".config/nvim/init.lua"));
    assert!(output.stdout.contains(".config/fish/config.fish"));
    assert!(output.stdout.contains(".config/ghostty/config"));
    assert!(output.stdout.contains("Wrote 4 files."));
    assert!(output.stdout.contains("hook: post-link.sh"));
    assert!(output.stdout.contains("Ran 2 hooks"));
}

#[test]
fn link_ignores_special_files() {
    conf::init();

    // OK.
    conf::create_file_in_configs("subdir/.git/config", None);
    conf::create_file_in_configs("subdir/.gitignore", None);
    // NOT OK.
    conf::create_file_in_configs(".gitignore", None);
    conf::create_file_in_configs(".git/config", None);
    // NOT OK, even in subdirectories.
    conf::create_file_in_configs("subdir/.deez", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK in sub-directories.
    assert!(symlink_exists_in_home("subdir/.git/config"));
    assert!(symlink_exists_in_home("subdir/.gitignore"));
    // NOT OK in root.
    assert!(!symlink_exists_in_home(".gitignore"));
    assert!(!symlink_exists_in_home(".git/config"));
    // NOT OK, even in subdirectories.
    assert!(!symlink_exists_in_home("subdir/.deez"));
}

#[test]
fn link_replaces_file_with_symlink() {
    conf::init();

    let file_in_configs = conf::create_file_in_configs("config_file.txt", Some("new"));
    conf::create_symlink_in_configs(
        "config_symlink.txt",
        Some(&file_in_configs.to_string_lossy()),
    );

    let file_in_home = conf::create_file_in_home("config_file.txt", Some("old"));
    let (symlink_in_home, _) = conf::create_symlink_in_home("config_symlink.txt", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Ensure the file in home is a symlink now.
    assert!(file_in_home.is_symlink());
    assert_eq!(read(&file_in_home), "new");

    // Ensure the symlink in home points to the updated target.
    assert!(symlink_in_home.is_symlink());
    assert_eq!(read(&symlink_in_home), "new");
}

#[test]
fn link_respects_ignore_patters() {
    conf::init();

    conf::create_file_in_configs("foo/a.txt", None);
    conf::create_file_in_configs("bar/b.txt", None);
    conf::create_file_in_configs("baz/c.txt", None);

    conf::create_file_in_configs(".ignore", Some("foo/*"));
    conf::create_file_in_configs(".gitignore", Some("bar/b.txt"));

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!symlink_exists_in_home("foo/a.txt"));
    assert!(!symlink_exists_in_home("bar/b.txt"));
    assert!(symlink_exists_in_home("baz/c.txt"));

    assert!(!symlink_exists_in_home(".ignore"));
    assert!(!symlink_exists_in_home(".gitignore"));
}

#[test]
fn link_looks_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", None);

    let output = run_in_dir(&["--verbose", "link"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(symlink_exists_in_home("foo/bar/baz.txt"));
}

/// This test is important because the implementation `skip()`s the
/// current dir (if we're looking in parents, _we know_ the current dir
/// isn't a root). This test ensures we're not skipping too far.
#[test]
fn link_looks_for_root_in_direct_parent() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar.txt", None);

    let output = run_in_dir(&["--verbose", "link"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(symlink_exists_in_home("foo/bar.txt"));
}

#[test]
fn link_hooks_are_executed() {
    conf::init();

    // (Add 'OK's to differentiate from verbose output).
    conf::create_executable_file_in_configs("pre-link", Some("echo 'pre-link OK'"));
    conf::create_executable_file_in_configs("pre-link.sh", Some("echo 'pre-link.sh OK'"));
    conf::create_executable_file_in_configs("post-link.sh", Some("echo 'post-link.sh OK'"));

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("pre-link OK\n"));
    assert!(output.stdout.contains("pre-link.sh OK\n"));
    assert!(output.stdout.contains("post-link.sh OK\n"));

    assert!(output.stdout.contains("Ran 3 hooks."));
}

#[test]
fn link_hooks_are_executed_in_configs_dir() {
    conf::init();

    conf::create_executable_file_in_configs("post-link.sh", Some(r#"echo "post-link.sh:$(pwd)""#));

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("post-link.sh:{CONFIGS}\n")));
}

#[test]
fn link_hooks_are_executed_in_order_of_file_name() {
    conf::init();

    conf::create_executable_file_in_configs("post-link.sh", None);
    conf::create_executable_file_in_configs("post-link.py", None);
    conf::create_executable_file_in_configs("post-link.001.py", None);
    conf::create_executable_file_in_configs("post-link.002.sh", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(
        "\
hook: post-link.001.py
hook: post-link.002.sh
hook: post-link.py
hook: post-link.sh
"
    ));
}

#[test]
fn link_hooks_ignore_other_commands_hooks() {
    conf::init();

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!output.stdout.contains("hook: pre-sync.sh"));
    assert!(!output.stdout.contains("hook: post-sync.sh"));
    assert!(!output.stdout.contains("hook: pre-rsync.sh"));
    assert!(!output.stdout.contains("hook: post-rsync.sh"));
    assert!(output.stdout.contains("hook: pre-link.sh"));
    assert!(output.stdout.contains("hook: post-link.sh"));
    assert!(!output.stdout.contains("hook: pre-status.sh"));
    assert!(!output.stdout.contains("hook: post-status.sh"));
}

#[test]
fn link_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.py", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.py", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.py", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!file_exists_in_home("pre-sync.sh"));
    assert!(!file_exists_in_home("post-sync.py"));
    assert!(!file_exists_in_home("pre-rsync.sh"));
    assert!(!file_exists_in_home("post-rsync.py"));
    assert!(!file_exists_in_home("pre-link.sh"));
    assert!(!file_exists_in_home("post-link.py"));
    assert!(!file_exists_in_home("pre-status.sh"));
    assert!(!file_exists_in_home("post-status.py"));
}

#[test]
fn link_hooks_expose_verbose_mode() {
    conf::init();

    conf::create_executable_file_in_configs(
        "pre-link.sh",
        Some(r#"[ -n "$DEEZ_VERBOSE" ] && echo verbose=true || echo verbose=false"#),
    );

    // Normal run.
    let output = run(&["link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=false"));

    // Verbose run.
    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=true"));
}

#[test]
fn link_hooks_are_not_copied_to_home() {
    conf::init();

    // Regular files.
    conf::create_file_in_configs("foo/pre-link.sh", None);
    conf::create_file_in_configs("foo/post-link.sh", None);

    // Hooks.
    conf::create_file_in_configs("pre-link.sh", None);
    conf::create_file_in_configs("post-link.sh", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Non-root "hooks" are not hooks, but regular files.
    assert!(file_exists_in_home("foo/pre-link.sh"));
    assert!(file_exists_in_home("foo/post-link.sh"));

    // Hooks are not copied.
    assert!(!file_exists_in_home("pre-link.sh"));
    assert!(!file_exists_in_home("post-link.sh"));
}
