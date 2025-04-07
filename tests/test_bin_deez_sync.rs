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
use std::path::PathBuf;

use conf::{CONFIGS, HOME};
use run::{run, run_in_dir};

fn file_exists_in_home(file_path: &str) -> bool {
    let file = PathBuf::from(HOME).join(file_path);
    file.is_file()
}

// fn symlink_exists_in_home(symlink_path: &str) -> bool {
//     let symlink = PathBuf::from(HOME).join(symlink_path);
//     symlink.is_symlink()
// }

#[test]
fn sync_regular() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(file_exists_in_home(".gitconfig"));
    assert!(file_exists_in_home(".config/nvim/init.lua"));
    assert!(file_exists_in_home(".config/fish/config.fish"));
    assert!(file_exists_in_home(".config/ghostty/config"));
}

#[test]
fn sync_ignores_special_files() {
    conf::init();

    // OK.
    conf::create_file_in_configs("subdir/.git/config", None);
    conf::create_file_in_configs("subdir/.gitignore", None);
    // NOT OK.
    conf::create_file_in_configs(".gitignore", None);
    conf::create_file_in_configs(".git/config", None);
    // NOT OK, even in subdirectories.
    conf::create_file_in_configs("subdir/.deez", None);

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK in sub-directories.
    assert!(file_exists_in_home("subdir/.git/config"));
    assert!(file_exists_in_home("subdir/.gitignore"));
    // NOT OK in root.
    assert!(!file_exists_in_home(".gitignore"));
    assert!(!file_exists_in_home(".git/config"));
    // NOT OK, even in subdirectories.
    assert!(!file_exists_in_home("subdir/.deez"));
}

/// If a file in configs should override a symlink in home, ensure `sync`
/// replaces the symlink with a file, and does _not_ replace the content
/// of the target of the symlink.
#[test]
fn sync_replace_symlink_with_file() {
    conf::init();

    // Real file in configs.
    conf::create_file_in_configs("config_file.txt", Some("new content"));

    // Target file that should _not_ be overridden.
    let symlink_target_in_home =
        conf::create_file_in_home("symlink_target.txt", Some("should not be replaced"));

    // Symlink in home.
    let (symlink_in_home, _) =
        conf::create_symlink_in_home("config_file.txt", Some("symlink_target.txt"));

    // Ensure the symlink in home links to target file.
    let content_before = fs::read_to_string(&symlink_in_home).unwrap();
    assert!(symlink_in_home.is_symlink());
    assert_eq!(content_before, "should not be replaced");

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Ensure the symlink in home is a file now, with updated content.
    let content_after = fs::read_to_string(&symlink_in_home).unwrap();
    assert!(!symlink_in_home.is_symlink()); // `is_file()` traverses.
    assert_eq!(content_after, "new content");

    // Ensure the removed symlink's target has not been altered.
    let symlink_target_content = fs::read_to_string(&symlink_target_in_home).unwrap();
    assert_eq!(symlink_target_content, "should not be replaced");
}

#[test]
fn sync_respect_ignore_patters() {
    conf::init();

    conf::create_file_in_configs("foo/a.txt", None);
    conf::create_file_in_configs("bar/b.txt", None);
    conf::create_file_in_configs("baz/c.txt", None);

    conf::create_file_in_configs(".ignore", Some("foo/*"));
    conf::create_file_in_configs(".gitignore", Some("bar/b.txt"));

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!file_exists_in_home("foo/a.txt"));
    assert!(!file_exists_in_home("bar/b.txt"));
    assert!(file_exists_in_home("baz/c.txt"));

    assert!(!file_exists_in_home(".ignore"));
    assert!(!file_exists_in_home(".gitignore"));
}

#[test]
fn sync_look_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", None);

    let output = run_in_dir(&["sync"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(file_exists_in_home("foo/bar/baz.txt"));
}

/// This test is important because the implementation `skip()`s the
/// current dir (if we're looking in parents, _we know_ the current dir
/// isn't a root). This test ensures we're not skipping too far.
#[test]
fn sync_look_for_root_in_direct_parent() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar.txt", None);

    let output = run_in_dir(&["sync"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(file_exists_in_home("foo/bar.txt"));
}

#[test]
fn sync_hooks_are_executed() {
    conf::init();

    // (Add 'OK's to differentiate from verbose output).
    conf::create_executable_file_in_configs("pre-sync", Some("echo 'pre-sync OK'"));
    conf::create_executable_file_in_configs("pre-sync.sh", Some("echo 'pre-sync.sh OK'"));
    conf::create_executable_file_in_configs("post-sync.sh", Some("echo 'post-sync.sh OK'"));

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("pre-sync OK\n"));
    assert!(output.stdout.contains("pre-sync.sh OK\n"));
    assert!(output.stdout.contains("post-sync.sh OK\n"));

    assert!(output.stdout.contains("Ran 3 hooks."));
}

#[test]
fn sync_hooks_are_executed_in_configs_dir() {
    conf::init();

    conf::create_executable_file_in_configs("post-sync.sh", Some(r#"echo "post-sync.sh:$(pwd)""#));

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("post-sync.sh:{CONFIGS}\n")));
}

#[test]
fn sync_hooks_are_not_copied_to_home() {
    conf::init();

    // Regular files.
    conf::create_file_in_configs("foo/pre-sync.sh", None);
    conf::create_file_in_configs("foo/post-sync.sh", None);

    // Hooks.
    conf::create_file_in_configs("pre-sync.sh", None);
    conf::create_file_in_configs("post-sync.sh", None);

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Non-root "hooks" are not hooks, but regular files.
    assert!(file_exists_in_home("foo/pre-sync.sh"));
    assert!(file_exists_in_home("foo/post-sync.sh"));

    // Hooks are not copied.
    assert!(!file_exists_in_home("pre-sync.sh"));
    assert!(!file_exists_in_home("post-sync.sh"));
}
