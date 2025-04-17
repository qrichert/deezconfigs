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

mod utils;

use utils::conf::{self, CONFIGS, HOME};
use utils::files::{dir_exists_in_home, file_exists_in_home};
use utils::run::{run, run_in_dir};

#[test]
fn clean_regular() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);

    conf::create_file_in_home(".gitconfig", None);
    conf::create_file_in_home(".config/nvim/init.lua", None);
    conf::create_file_in_home(".config/fish/config.fish", None);
    conf::create_symlink_in_home(".config/ghostty/config", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!file_exists_in_home(".gitconfig"));
    assert!(!file_exists_in_home(".config/nvim/init.lua"));
    assert!(!file_exists_in_home(".config/fish/config.fish"));
    assert!(!file_exists_in_home(".config/ghostty/config"));
}

#[test]
fn clean_output() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-clean.sh", None);
    conf::create_executable_file_in_configs("post-clean.sh", None);

    conf::create_file_in_home(".gitconfig", None);
    conf::create_file_in_home(".config/nvim/init.lua", None);
    conf::create_file_in_home(".config/fish/config.fish", None);
    conf::create_symlink_in_home(".config/ghostty/config", None);

    let output = run(&["clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
Removed 4 files.
Ran 2 hooks.
"
    );
}

#[test]
fn clean_output_verbose() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-clean.sh", None);
    conf::create_executable_file_in_configs("post-clean.sh", None);

    conf::create_file_in_home(".gitconfig", None);
    conf::create_file_in_home(".config/nvim/init.lua", None);
    conf::create_file_in_home(".config/fish/config.fish", None);
    conf::create_symlink_in_home(".config/ghostty/config", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
hook: pre-clean.sh
.config/fish/config.fish
.config/ghostty/config
.config/nvim/init.lua
.gitconfig
hook: post-clean.sh
Removed 4 files.
Ran 2 hooks.
"
    );
}

#[test]
fn clean_cleans_up_directories_left_empty() {
    conf::init();

    conf::create_file_in_configs("deeply/nested/file/foo.txt", None);

    conf::create_file_in_home("deeply/nested/file/foo.txt", None);
    conf::create_file_in_home("deeply/is-not-empty.txt", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Both `file/` and `nested/` are left empty and should be deleted.
    assert!(!dir_exists_in_home("deeply/nested/"));
    // `deeply/` still contains a file and should _not_ be deleted.
    assert!(dir_exists_in_home("deeply/"));
}

#[test]
fn clean_cleans_up_directories_left_empty_but_not_home_and_above() {
    conf::init();

    conf::create_file_in_configs("deeply/nested/file/foo.txt", None);

    conf::create_file_in_home("deeply/nested/file/foo.txt", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // The whole subtree is empty and should be deleted.
    assert!(!file_exists_in_home("deeply/nested/file/foo.txt"));

    // Home itself still exists.
    assert!(dir_exists_in_home("./"));
}

#[test]
fn clean_ignores_special_files() {
    conf::init();

    // OK.
    conf::create_file_in_configs("subdir/.git/config", None);
    conf::create_file_in_configs("subdir/.gitignore", None);
    // NOT OK.
    conf::create_file_in_configs(".gitignore", None);
    conf::create_file_in_configs(".git/config", None);
    // NOT OK, even in subdirectories.
    conf::create_file_in_configs("subdir/.deez", None);

    // OK.
    conf::create_file_in_home("subdir/.git/config", None);
    conf::create_file_in_home("subdir/.gitignore", None);
    // NOT OK.
    conf::create_file_in_home(".gitignore", None);
    conf::create_file_in_home(".git/config", None);
    // NOT OK, even in subdirectories.
    conf::create_file_in_home("subdir/.deez", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK in sub-directories.
    assert!(!file_exists_in_home("subdir/.git/config"));
    assert!(!file_exists_in_home("subdir/.gitignore"));
    // NOT OK in root.
    assert!(file_exists_in_home(".gitignore"));
    assert!(file_exists_in_home(".git/config"));
    // NOT OK, even in subdirectories.
    assert!(file_exists_in_home("subdir/.deez"));
}

#[test]
fn clean_respects_ignore_patters() {
    conf::init();

    conf::create_file_in_configs("foo/a.txt", None);
    conf::create_file_in_configs("bar/b.txt", None);
    conf::create_file_in_configs("baz/c.txt", None);

    conf::create_file_in_configs(".ignore", Some("foo/*"));
    conf::create_file_in_configs(".gitignore", Some("bar/b.txt"));

    conf::create_file_in_home("foo/a.txt", None);
    conf::create_file_in_home("bar/b.txt", None);
    conf::create_file_in_home("baz/c.txt", None);

    conf::create_file_in_home(".ignore", Some("foo/*"));
    conf::create_file_in_home(".gitignore", Some("bar/b.txt"));

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(file_exists_in_home("foo/a.txt"));
    assert!(file_exists_in_home("bar/b.txt"));
    assert!(!file_exists_in_home("baz/c.txt"));

    assert!(file_exists_in_home(".ignore"));
    assert!(file_exists_in_home(".gitignore"));
}

#[test]
fn clean_looks_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", None);
    conf::create_file_in_home("foo/bar/baz.txt", None);

    let output = run_in_dir(&["--verbose", "clean"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!file_exists_in_home("foo/bar/baz.txt"));
}

/// This test is important because the implementation `skip()`s the
/// current dir (if we're looking in parents, _we know_ the current dir
/// isn't a root). This test ensures we're not skipping too far.
#[test]
fn clean_looks_for_root_in_direct_parent() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar.txt", None);
    conf::create_file_in_home("foo/bar.txt", None);

    let output = run_in_dir(&["--verbose", "clean"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!file_exists_in_home("foo/bar.txt"));
}

#[test]
fn clean_hooks_are_executed() {
    conf::init();

    // (Add 'OK's to differentiate from verbose output).
    conf::create_executable_file_in_configs("pre-clean", Some("echo 'pre-clean OK'"));
    conf::create_executable_file_in_configs("pre-clean.sh", Some("echo 'pre-clean.sh OK'"));
    conf::create_executable_file_in_configs("post-clean.sh", Some("echo 'post-clean.sh OK'"));

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("pre-clean OK\n"));
    assert!(output.stdout.contains("pre-clean.sh OK\n"));
    assert!(output.stdout.contains("post-clean.sh OK\n"));

    assert!(output.stdout.contains("Ran 3 hooks."));
}

#[test]
fn clean_hooks_are_executed_in_configs_dir() {
    conf::init();

    conf::create_executable_file_in_configs(
        "post-clean.sh",
        Some(r#"echo "post-clean.sh:$(pwd)""#),
    );

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(
        output
            .stdout
            .contains(&format!("post-clean.sh:{CONFIGS}\n"))
    );
}

#[test]
fn clean_hooks_are_executed_in_order_of_file_name() {
    conf::init();

    conf::create_executable_file_in_configs("post-clean.sh", None);
    conf::create_executable_file_in_configs("post-clean.py", None);
    conf::create_executable_file_in_configs("post-clean.001.py", None);
    conf::create_executable_file_in_configs("post-clean.002.sh", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(
        "\
hook: post-clean.001.py
hook: post-clean.002.sh
hook: post-clean.py
hook: post-clean.sh
"
    ));
}

#[test]
fn clean_hooks_ignore_other_commands_hooks() {
    conf::init();

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);
    conf::create_executable_file_in_configs("pre-clean.sh", None);
    conf::create_executable_file_in_configs("post-clean.sh", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!output.stdout.contains("hook: pre-sync.sh"));
    assert!(!output.stdout.contains("hook: post-sync.sh"));
    assert!(!output.stdout.contains("hook: pre-rsync.sh"));
    assert!(!output.stdout.contains("hook: post-rsync.sh"));
    assert!(!output.stdout.contains("hook: pre-link.sh"));
    assert!(!output.stdout.contains("hook: post-link.sh"));
    assert!(!output.stdout.contains("hook: pre-status.sh"));
    assert!(!output.stdout.contains("hook: post-status.sh"));
    assert!(output.stdout.contains("hook: pre-clean.sh"));
    assert!(output.stdout.contains("hook: post-clean.sh"));
}

#[test]
fn clean_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.py", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.py", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.py", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);
    conf::create_executable_file_in_configs("pre-clean.sh", None);
    conf::create_executable_file_in_configs("post-clean.sh", None);

    conf::create_file_in_home("pre-sync.sh", None);
    conf::create_file_in_home("post-sync.py", None);
    conf::create_file_in_home("pre-rsync.sh", None);
    conf::create_file_in_home("post-rsync.py", None);
    conf::create_file_in_home("pre-link.sh", None);
    conf::create_file_in_home("post-link.py", None);
    conf::create_file_in_home("pre-status.sh", None);
    conf::create_file_in_home("post-status.sh", None);
    conf::create_file_in_home("pre-clean.sh", None);
    conf::create_file_in_home("post-clean.sh", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(file_exists_in_home("pre-sync.sh"));
    assert!(file_exists_in_home("post-sync.py"));
    assert!(file_exists_in_home("pre-rsync.sh"));
    assert!(file_exists_in_home("post-rsync.py"));
    assert!(file_exists_in_home("pre-link.sh"));
    assert!(file_exists_in_home("post-link.py"));
    assert!(file_exists_in_home("pre-status.sh"));
    assert!(file_exists_in_home("post-status.sh"));
    assert!(file_exists_in_home("pre-clean.sh"));
    assert!(file_exists_in_home("post-clean.sh"));
}

#[test]
fn clean_hooks_expose_root() {
    conf::init();

    conf::create_executable_file_in_configs("pre-clean.sh", Some(r"echo root=$DEEZ_ROOT"));

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("\nroot={CONFIGS}\n")));
}

#[test]
fn clean_hooks_expose_home() {
    conf::init();

    conf::create_executable_file_in_configs("pre-clean.sh", Some(r"echo home=$DEEZ_HOME"));

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("\nhome={HOME}\n")));
}

#[test]
fn clean_hooks_expose_verbose_mode() {
    conf::init();

    conf::create_executable_file_in_configs(
        "pre-clean.sh",
        Some(r#"[ -n "$DEEZ_VERBOSE" ] && echo verbose=true || echo verbose=false"#),
    );

    // Normal run.
    let output = run(&["clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=false"));

    // Verbose run.
    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=true"));
}
