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

use std::env;
use std::path::Path;

use utils::conf::{self, CONFIGS, HOME};
use utils::run::{run, run_in_dir};

#[test]
fn diff_regular() {
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

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
bar/baz.txt
@@ -1,1 +1,1 @@
-this is bar/baz
+not equal

biz.txt
@@ -1,1 +1,1 @@
-this is bar/baz
+not equal

boz.txt
! File does not exist in Home.
! Skipping...
"
    );
}

#[test]
fn diff_reversed() {
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

    let output = run(&["--verbose", "diff", &conf::root(), "--reversed"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
bar/baz.txt
@@ -1,1 +1,1 @@
-not equal
+this is bar/baz

biz.txt
@@ -1,1 +1,1 @@
-not equal
+this is bar/baz

boz.txt
! File does not exist in Home.
! Skipping...
"
    );
}

#[test]
fn diff_with_hooks() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);
    conf::create_executable_file_in_configs("pre-diff.sh", None);
    conf::create_executable_file_in_configs("post-diff.sh", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
hook: pre-diff.sh
hook: post-diff.sh
foo.txt
! File does not exist in Home.
! Skipping...
Ran 2 hooks.
"
    );
}

#[test]
fn diff_ignores_special_files() {
    conf::init();

    // OK.
    conf::create_file_in_configs("subdir/.git/config", None);
    // NOT OK.
    conf::create_file_in_configs(".ignore", None);
    conf::create_file_in_configs("subdir/.ignore", None);
    conf::create_file_in_configs(".gitignore", None);
    conf::create_file_in_configs(".git/config", None);
    conf::create_file_in_configs("subdir/.gitignore", None);
    conf::create_file_in_configs("subdir/.deez", None);

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK.
    assert!(output.stdout.contains("subdir/.git/config"));
    // NOT OK.
    assert!(!output.stdout.contains("\n.ignore"));
    assert!(!output.stdout.contains("subdir/.ignore"));
    assert!(!output.stdout.contains("\n.gitignore"));
    assert!(!output.stdout.contains("\n.git/config"));
    assert!(!output.stdout.contains("subdir/.gitignore"));
    assert!(!output.stdout.contains("subdir/.deez"));
}

#[test]
fn diff_respects_ignore_patters() {
    conf::init();

    conf::create_file_in_configs("foo/a.txt", None);
    conf::create_file_in_configs("bar/b.txt", None);
    conf::create_file_in_configs("baz/c.txt", None);

    conf::create_file_in_configs(".ignore", Some("foo/*"));
    conf::create_file_in_configs(".gitignore", Some("bar/b.txt"));

    let output = run(&["--verbose", "diff", &conf::root()]);
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
fn diff_looks_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", None);

    let output = run_in_dir(&["--verbose", "diff"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("foo/bar/baz.txt"));
}

/// This test is important because the implementation `skip()`s the
/// current dir (if we're looking in parents, _we know_ the current dir
/// isn't a root). This test ensures we're not skipping too far.
#[test]
fn diff_looks_for_root_in_direct_parent() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar.txt", None);

    let output = run_in_dir(&["--verbose", "diff"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("foo/bar.txt"));
}

#[test]
fn diff_uses_deez_root_variable_if_no_root_specified() {
    conf::init();

    conf::create_file_in_configs("bar.txt", None);

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    // Run outside of any root. It should use `DEEZ_ROOT`.
    let output = run_in_dir(
        &["--verbose", "diff"],
        Path::new(&conf::root()).parent().unwrap(),
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("bar.txt"));
}

#[test]
fn diff_hooks_are_executed() {
    conf::init();

    // (Add 'OK's to differentiate from verbose output).
    conf::create_executable_file_in_configs("pre-diff", Some("echo 'pre-diff OK'"));
    conf::create_executable_file_in_configs("pre-diff.sh", Some("echo 'pre-diff.sh OK'"));
    conf::create_executable_file_in_configs("post-diff.sh", Some("echo 'post-diff.sh OK'"));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("pre-diff OK\n"));
    assert!(output.stdout.contains("pre-diff.sh OK\n"));
    assert!(output.stdout.contains("post-diff.sh OK\n"));

    assert!(output.stdout.contains("Ran 3 hooks."));
}

#[test]
fn diff_hooks_are_executed_in_configs_dir() {
    conf::init();

    conf::create_executable_file_in_configs("post-diff.sh", Some(r#"echo "post-diff.sh:$(pwd)""#));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("post-diff.sh:{CONFIGS}\n")));
}

#[test]
fn diff_hooks_are_executed_in_order_of_file_name() {
    conf::init();

    conf::create_executable_file_in_configs("post-diff.sh", None);
    conf::create_executable_file_in_configs("post-diff.py", None);
    conf::create_executable_file_in_configs("post-diff.001.py", None);
    conf::create_executable_file_in_configs("post-diff.002.sh", None);

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(
        "\
hook: post-diff.001.py
hook: post-diff.002.sh
hook: post-diff.py
hook: post-diff.sh
"
    ));
}

#[test]
fn diff_hooks_ignore_other_commands_hooks() {
    conf::init();

    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);
    conf::create_executable_file_in_configs("pre-rsync.sh", None);
    conf::create_executable_file_in_configs("post-rsync.sh", None);
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);
    conf::create_executable_file_in_configs("pre-status.sh", None);
    conf::create_executable_file_in_configs("post-status.sh", None);
    conf::create_executable_file_in_configs("post-diff.sh", None);
    conf::create_executable_file_in_configs("pre-diff.sh", None);
    conf::create_executable_file_in_configs("pre-clean.sh", None);
    conf::create_executable_file_in_configs("post-clean.sh", None);

    let output = run(&["--verbose", "diff", &conf::root()]);
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
    assert!(output.stdout.contains("hook: pre-diff.sh"));
    assert!(output.stdout.contains("hook: post-diff.sh"));
    assert!(!output.stdout.contains("hook: pre-clean.sh"));
    assert!(!output.stdout.contains("hook: post-clean.sh"));
}

#[test]
fn diff_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);
    conf::create_file_in_home("foo", None);

    conf::create_executable_file_in_configs("pre-sync.sh", Some("# pre"));
    conf::create_executable_file_in_configs("post-sync.py", Some("# post"));
    conf::create_executable_file_in_configs("pre-rsync.sh", Some("# pre"));
    conf::create_executable_file_in_configs("post-rsync.py", Some("# post"));
    conf::create_executable_file_in_configs("pre-link.sh", Some("# pre"));
    conf::create_executable_file_in_configs("post-link.py", Some("# post"));
    conf::create_executable_file_in_configs("pre-status.sh", Some("# pre"));
    conf::create_executable_file_in_configs("post-status.sh", Some("# post"));
    conf::create_executable_file_in_configs("pre-diff.sh", Some("# pre"));
    conf::create_executable_file_in_configs("post-diff.sh", Some("# post"));
    conf::create_executable_file_in_configs("pre-clean.sh", Some("# pre"));
    conf::create_executable_file_in_configs("post-clean.sh", Some("# post"));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(
        "\
hook: pre-diff.sh
hook: post-diff.sh
Home is in sync.
Ran 2 hooks.
"
    ));
}

#[test]
fn diff_hooks_expose_root() {
    conf::init();

    conf::create_executable_file_in_configs("pre-diff.sh", Some(r"echo root=$DEEZ_ROOT"));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("\nroot={CONFIGS}\n")));
}

#[test]
fn diff_hooks_expose_home() {
    conf::init();

    conf::create_executable_file_in_configs("pre-diff.sh", Some(r"echo home=$DEEZ_HOME"));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("\nhome={HOME}\n")));
}

#[test]
fn diff_hooks_expose_verbose_mode() {
    conf::init();

    conf::create_executable_file_in_configs(
        "pre-diff.sh",
        Some(r#"[ -n "$DEEZ_VERBOSE" ] && echo verbose=true || echo verbose=false"#),
    );

    // Normal run.
    let output = run(&["diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=false"));

    // Verbose run.
    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("verbose=true"));
}

#[test]
fn diff_hooks_expose_os() {
    conf::init();

    conf::create_executable_file_in_configs("pre-diff.sh", Some(r"echo os=$DEEZ_OS"));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(
        output
            .stdout
            .contains(&format!("\nos={}\n", std::env::consts::OS))
    );
}

#[test]
fn diff_hooks_abort_execution_if_exit_code_is_non_zero() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);

    conf::create_executable_file_in_configs("pre-diff.sh", Some(r"exit 1"));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 1);

    assert!(!output.stdout.contains(".gitconfig"));
    assert!(
        output
            .stderr
            .contains("abort: Execution aborted by 'pre-diff.sh'.")
    );
}
