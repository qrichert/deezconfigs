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

#![allow(clippy::many_single_char_names)]

mod conf;
mod run;

use std::fs;
use std::path::Path;

use conf::CONFIGS;
use run::{run, run_in_dir};

fn read(file_path: &Path) -> String {
    fs::read_to_string(file_path).unwrap()
}

#[test]
fn rsync_regular() {
    conf::init();

    let git = conf::create_file_in_configs(".gitconfig", Some("old"));
    let nvim = conf::create_file_in_configs(".config/nvim/init.lua", Some("old"));
    let fish = conf::create_file_in_configs(".config/fish/config.fish", Some("old"));
    let ghostty = conf::create_file_in_configs(".config/ghostty/config", Some("old"));

    conf::create_file_in_home(".gitconfig", Some("new"));
    conf::create_file_in_home(".config/nvim/init.lua", Some("new"));
    conf::create_file_in_home(".config/fish/config.fish", Some("new"));
    let ghostty_target = conf::create_file_in_home("ghostty_target", Some("new"));
    conf::create_symlink_in_home(
        ".config/ghostty/config",
        Some(&ghostty_target.to_string_lossy()),
    );

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read(&git), "new");
    assert_eq!(read(&nvim), "new");
    assert_eq!(read(&fish), "new");
    assert_eq!(read(&ghostty), "new");
}

#[test]
fn rsync_output() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);

    conf::create_file_in_home(".gitconfig", None);
    conf::create_file_in_home(".config/nvim/init.lua", None);
    conf::create_file_in_home(".config/fish/config.fish", None);
    conf::create_symlink_in_home(".config/ghostty/config", None);

    let output = run(&["rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // File order is non-deterministic.
    assert!(!output.stdout.contains("hook: pre-rsync.sh"));
    assert!(!output.stdout.contains(".gitconfig"));
    assert!(!output.stdout.contains(".config/nvim/init.lua"));
    assert!(!output.stdout.contains(".config/fish/config.fish"));
    assert!(!output.stdout.contains(".config/ghostty/config"));
    assert!(output.stdout.contains("Wrote 4 files."));
    assert!(!output.stdout.contains("hook: post-rsync.sh"));
    assert!(output.stdout.contains("Ran 2 hooks"));
}

#[test]
fn rsync_output_verbose() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);

    conf::create_file_in_home(".gitconfig", None);
    conf::create_file_in_home(".config/nvim/init.lua", None);
    conf::create_file_in_home(".config/fish/config.fish", None);
    conf::create_symlink_in_home(".config/ghostty/config", None);

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // File order is non-deterministic.
    assert!(output.stdout.contains("hook: pre-rsync.sh"));
    assert!(output.stdout.contains(".gitconfig"));
    assert!(output.stdout.contains(".config/nvim/init.lua"));
    assert!(output.stdout.contains(".config/fish/config.fish"));
    assert!(output.stdout.contains(".config/ghostty/config"));
    assert!(output.stdout.contains("Wrote 4 files."));
    assert!(output.stdout.contains("hook: post-rsync.sh"));
    assert!(output.stdout.contains("Ran 2 hooks"));
}

#[test]
fn rsync_ignores_special_files() {
    conf::init();

    // OK.
    let sub_gitconfig = conf::create_file_in_configs("subdir/.git/config", Some("old"));
    let sub_gitignore = conf::create_file_in_configs("subdir/.gitignore", Some("old"));
    // NOT OK.
    let gitignore = conf::create_file_in_configs(".gitignore", Some("old"));
    let gitconfig = conf::create_file_in_configs(".git/config", Some("old"));
    // NOT OK, even in subdirectories.
    let sub_deez = conf::create_file_in_configs("subdir/.deez", Some("old"));

    // OK.
    conf::create_file_in_home("subdir/.git/config", Some("new"));
    conf::create_file_in_home("subdir/.gitignore", Some("new"));
    // NOT OK.
    conf::create_file_in_home(".gitignore", Some("new"));
    conf::create_file_in_home(".git/config", Some("new"));
    // NOT OK, even in subdirectories.
    conf::create_file_in_home("subdir/.deez", Some("new"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK in sub-directories.
    assert_eq!(read(&sub_gitconfig), "new");
    assert_eq!(read(&sub_gitignore), "new");
    // NOT OK in root.
    assert_eq!(read(&gitignore), "old");
    assert_eq!(read(&gitconfig), "old");
    // NOT OK, even in subdirectories.
    assert_eq!(read(&sub_deez), "old");
}

/// If a file in home should override a symlink in configs, ensure
/// `rsync` replaces the symlink with a file, and does _not_ replace the
/// content of the target of the symlink.
#[test]
fn rsync_replaces_symlink_with_file() {
    conf::init();

    // Real file in home.
    conf::create_file_in_home("config_file.txt", Some("new content"));

    // Target file that should _not_ be overridden.
    let symlink_target_in_configs =
        conf::create_file_in_configs("symlink_target.txt", Some("should not be replaced"));
    conf::create_file_in_configs(".ignore", Some("symlink_target.txt"));

    // Symlink in configs.
    let (symlink_in_configs, _) =
        conf::create_symlink_in_configs("config_file.txt", Some("symlink_target.txt"));

    // Ensure the symlink in configs links to target file.
    assert!(symlink_in_configs.is_symlink());
    assert_eq!(read(&symlink_in_configs), "should not be replaced");

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Ensure the symlink in configs is a file now, with updated content.
    assert!(!symlink_in_configs.is_symlink()); // `is_file()` traverses.
    assert_eq!(read(&symlink_in_configs), "new content");

    // Ensure the removed symlink's target has not been altered.
    assert_eq!(read(&symlink_target_in_configs), "should not be replaced");
}

#[test]
fn rsync_respects_ignore_patters() {
    conf::init();

    let a = conf::create_file_in_configs("foo/a.txt", Some("old"));
    let b = conf::create_file_in_configs("bar/b.txt", Some("old"));
    let c = conf::create_file_in_configs("baz/c.txt", Some("old"));

    let ignore = conf::create_file_in_configs(".ignore", Some("foo/*"));
    let gitignore = conf::create_file_in_configs(".gitignore", Some("bar/b.txt"));

    conf::create_file_in_home("foo/a.txt", Some("new"));
    conf::create_file_in_home("bar/b.txt", Some("new"));
    conf::create_file_in_home("baz/c.txt", Some("new"));

    conf::create_file_in_home(".ignore", Some("new"));
    conf::create_file_in_home(".gitignore", Some("new"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read(&a), "old");
    assert_eq!(read(&b), "old");
    assert_eq!(read(&c), "new");

    assert_ne!(read(&ignore), "new");
    assert_ne!(read(&gitignore), "new");
}

#[test]
fn rsync_looks_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", Some("old"));

    conf::create_file_in_home("foo/bar/baz.txt", Some("new"));

    let output = run_in_dir(&["--verbose", "rsync"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read(&file), "new");
}

/// This test is important because the implementation `skip()`s the
/// current dir (if we're looking in parents, _we know_ the current dir
/// isn't a root). This test ensures we're not skipping too far.
#[test]
fn rsync_looks_for_root_in_direct_parent() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar.txt", Some("old"));

    conf::create_file_in_home("foo/bar.txt", Some("new"));

    let output = run_in_dir(&["--verbose", "rsync"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read(&file), "new");
}

#[test]
fn rsync_hooks_are_executed() {
    conf::init();

    // (Add 'OK's to differentiate from verbose output).
    conf::create_executable_file_in_configs("pre-rsync", Some("echo 'pre-rsync OK'"));
    conf::create_executable_file_in_configs("pre-rsync.sh", Some("echo 'pre-rsync.sh OK'"));
    conf::create_executable_file_in_configs("post-rsync.sh", Some("echo 'post-rsync.sh OK'"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("pre-rsync OK\n"));
    assert!(output.stdout.contains("pre-rsync.sh OK\n"));
    assert!(output.stdout.contains("post-rsync.sh OK\n"));

    assert!(output.stdout.contains("Ran 3 hooks."));
}

#[test]
fn rsync_hooks_are_executed_in_configs_dir() {
    conf::init();

    conf::create_executable_file_in_configs(
        "post-rsync.sh",
        Some(r#"echo "post-rsync.sh:$(pwd)""#),
    );

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(
        output
            .stdout
            .contains(&format!("post-rsync.sh:{CONFIGS}\n"))
    );
}

#[test]
fn rsync_hooks_are_executed_in_order_of_file_name() {
    conf::init();

    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.py", None);
    conf::create_executable_file_in_configs("post-rsync.001.py", None);
    conf::create_executable_file_in_configs("post-rsync.002.sh", None);

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(
        "\
hook: post-rsync.001.py
hook: post-rsync.002.sh
hook: post-rsync.py
hook: post-rsync.sh
"
    ));
}

#[test]
fn rsync_hooks_ignore_other_commands_hooks() {
    conf::init();

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!output.stdout.contains("hook: pre-sync.sh"));
    assert!(!output.stdout.contains("hook: post-sync.sh"));
    assert!(output.stdout.contains("hook: pre-rsync.sh"));
    assert!(output.stdout.contains("hook: post-rsync.sh"));
    assert!(!output.stdout.contains("hook: pre-link.sh"));
    assert!(!output.stdout.contains("hook: post-link.sh"));
}

#[test]
fn rsync_hooks_are_not_treated_as_config_files() {
    conf::init();

    let a = conf::create_executable_file_in_configs("pre-sync.sh", Some("# old"));
    let b = conf::create_executable_file_in_configs("post-sync.py", Some("# old"));
    let c = conf::create_executable_file_in_configs("pre-rsync.sh", Some("# old"));
    let d = conf::create_executable_file_in_configs("post-rsync.sh", Some("# old"));
    let e = conf::create_executable_file_in_configs("pre-link.sh", Some("# old"));
    let f = conf::create_executable_file_in_configs("post-link.py", Some("# old"));

    conf::create_file_in_home("pre-sync.sh", Some("new"));
    conf::create_file_in_home("post-sync.py", Some("new"));
    conf::create_file_in_home("pre-rsync.sh", Some("new"));
    conf::create_file_in_home("post-rsync.py", Some("new"));
    conf::create_file_in_home("pre-link.sh", Some("new"));
    conf::create_file_in_home("post-link.py", Some("new"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read(&a), "# old");
    assert_eq!(read(&b), "# old");
    assert_eq!(read(&c), "# old");
    assert_eq!(read(&d), "# old");
    assert_eq!(read(&e), "# old");
    assert_eq!(read(&f), "# old");
}

#[test]
fn rsync_hooks_expose_verbose_mode() {
    conf::init();

    conf::create_executable_file_in_configs(
        "pre-rsync.sh",
        Some(r#"[ -n "$DEEZ_VERBOSE" ] && echo verbose=true || echo verbose=false"#),
    );

    // Normal run.
    let output = run(&["rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=false"));

    // Verbose run.
    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=true"));
}

#[test]
fn rsync_hooks_are_not_copied_from_home() {
    conf::init();

    // Regular files (configs).
    let sub_pre = conf::create_file_in_configs("foo/pre-rsync.sh", Some("old"));
    let sub_post = conf::create_file_in_configs("foo/post-rsync.sh", Some("old"));

    // Hooks (configs).
    let pre = conf::create_file_in_configs("pre-rsync.sh", Some("old"));
    let post = conf::create_file_in_configs("post-rsync.sh", Some("old"));

    // Regular files (home).
    conf::create_file_in_home("foo/pre-rsync.sh", Some("new"));
    conf::create_file_in_home("foo/post-rsync.sh", Some("new"));

    // Hooks (home).
    conf::create_file_in_home("pre-rsync.sh", Some("new"));
    conf::create_file_in_home("post-rsync.sh", Some("new"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Non-root "hooks" are not hooks, but regular files.
    assert_eq!(read(&sub_pre), "new");
    assert_eq!(read(&sub_post), "new");

    // Hooks are not copied.
    assert_eq!(read(&pre), "old");
    assert_eq!(read(&post), "old");
}
