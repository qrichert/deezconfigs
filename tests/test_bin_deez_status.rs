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

use conf::CONFIGS;
use run::{run, run_in_dir};

#[test]
fn status_regular() {
    conf::init();

    conf::create_file_in_configs("foo.txt", Some("this is foo"));
    conf::create_file_in_configs("bar/baz.txt", Some("this is bar/baz"));
    conf::create_symlink_in_configs("biz.txt", Some("bar/baz.txt"));
    conf::create_symlink_in_configs("buz.txt", Some("foo.txt"));
    conf::create_file_in_configs("boz.txt", None);

    conf::create_file_in_home("foo.txt", Some("this is foo")); // Equal.
    conf::create_file_in_home("bar/baz.txt", Some("not equal")); // Different.
    conf::create_symlink_in_home("biz.txt", Some("bar/baz.txt")); // Symlink to different.
    conf::create_symlink_in_home("buz.txt", Some("foo.txt")); // Symlink to equal.
    // conf::create_file_in_home("boz.txt", None); // Missing.

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
Files
  M  bar/baz.txt
  M  biz.txt@
  !  boz.txt
  S  buz.txt@
  S  foo.txt
"
    );
}

#[test]
fn status_with_hooks() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
hook: pre-status.sh
Files
  !  foo.txt
Hooks
  pre-sync.sh
  post-sync.sh
  pre-rsync.sh
  post-rsync.sh
  pre-link.sh
  post-link.sh
  pre-status.sh
  post-status.sh
hook: post-status.sh
Ran 2 hooks.
"
    );
}

#[test]
fn status_ignores_special_files() {
    conf::init();

    // OK.
    conf::create_file_in_configs("subdir/.git/config", None);
    conf::create_file_in_configs("subdir/.gitignore", None);
    // NOT OK.
    conf::create_file_in_configs(".gitignore", None);
    conf::create_file_in_configs(".git/config", None);
    // NOT OK, even in subdirectories.
    conf::create_file_in_configs("subdir/.deez", None);

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK in sub-directories.
    assert!(output.stdout.contains("subdir/.git/config"));
    assert!(output.stdout.contains("subdir/.gitignore"));
    // NOT OK in root.
    assert!(!output.stdout.contains(" .gitignore"));
    assert!(!output.stdout.contains(" .git/config"));
    // NOT OK, even in subdirectories.
    assert!(!output.stdout.contains("subdir/.deez"));
}

#[test]
fn status_respects_ignore_patters() {
    conf::init();

    conf::create_file_in_configs("foo/a.txt", None);
    conf::create_file_in_configs("bar/b.txt", None);
    conf::create_file_in_configs("baz/c.txt", None);

    conf::create_file_in_configs(".ignore", Some("foo/*"));
    conf::create_file_in_configs(".gitignore", Some("bar/b.txt"));

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!output.stdout.contains("foo/a.txt"));
    assert!(!output.stdout.contains("bar/b.txt"));
    assert!(output.stdout.contains("baz/c.txt"));

    assert!(!output.stdout.contains(".ignore"));
    assert!(!output.stdout.contains(".gitignore"));
}

#[test]
fn status_looks_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", None);

    let output = run_in_dir(&["--verbose", "status"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("foo/bar/baz.txt"));
}

/// This test is important because the implementation `skip()`s the
/// current dir (if we're looking in parents, _we know_ the current dir
/// isn't a root). This test ensures we're not skipping too far.
#[test]
fn status_looks_for_root_in_direct_parent() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar.txt", None);

    let output = run_in_dir(&["--verbose", "status"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("foo/bar.txt"));
}

#[test]
fn status_hooks_are_executed() {
    conf::init();

    // (Add 'OK's to differentiate from verbose output).
    conf::create_executable_file_in_configs("pre-status", Some("echo 'pre-status OK'"));
    conf::create_executable_file_in_configs("pre-status.sh", Some("echo 'pre-status.sh OK'"));
    conf::create_executable_file_in_configs("post-status.sh", Some("echo 'post-status.sh OK'"));

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("pre-status OK\n"));
    assert!(output.stdout.contains("pre-status.sh OK\n"));
    assert!(output.stdout.contains("post-status.sh OK\n"));

    assert!(output.stdout.contains("Ran 3 hooks."));
}

#[test]
fn status_hooks_are_executed_in_configs_dir() {
    conf::init();

    conf::create_executable_file_in_configs(
        "post-status.sh",
        Some(r#"echo "post-status.sh:$(pwd)""#),
    );

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(
        output
            .stdout
            .contains(&format!("post-status.sh:{CONFIGS}\n"))
    );
}

#[test]
fn status_hooks_are_executed_in_order_of_file_name() {
    conf::init();

    conf::create_executable_file_in_configs("post-status.sh", None);
    conf::create_executable_file_in_configs("post-status.py", None);
    conf::create_executable_file_in_configs("post-status.001.py", None);
    conf::create_executable_file_in_configs("post-status.002.sh", None);

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(
        "\
hook: post-status.001.py
hook: post-status.002.sh
hook: post-status.py
hook: post-status.sh
"
    ));
}

#[test]
fn status_hooks_ignore_other_commands_hooks() {
    conf::init();

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!output.stdout.contains("hook: pre-sync.sh"));
    assert!(!output.stdout.contains("hook: post-sync.sh"));
    assert!(!output.stdout.contains("hook: pre-rsync.sh"));
    assert!(!output.stdout.contains("hook: post-rsync.sh"));
    assert!(!output.stdout.contains("hook: pre-link.sh"));
    assert!(!output.stdout.contains("hook: post-link.sh"));
    assert!(output.stdout.contains("hook: pre-status.sh"));
    assert!(output.stdout.contains("hook: post-status.sh"));
}

#[test]
fn status_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.py", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.py", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.py", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(
        "\
Hooks
  pre-sync.sh
  post-sync.py
  pre-rsync.sh
  post-rsync.py
  pre-link.sh
  post-link.py
  pre-status.sh
  post-status.sh
"
    ));
}

#[test]
fn status_hooks_expose_verbose_mode() {
    conf::init();

    conf::create_executable_file_in_configs(
        "pre-status.sh",
        Some(r#"[ -n "$DEEZ_VERBOSE" ] && echo verbose=true || echo verbose=false"#),
    );

    // Normal run.
    let output = run(&["status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=false"));

    // Verbose run.
    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=true"));
}
