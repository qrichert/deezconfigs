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

#[path = "utils/hook_macros.rs"]
mod hook_macros;

// Generate shared hook tests for this command.
hook_macros::hook_tests!(diff);

use std::env;
use std::path::Path;

use utils::conf;
use utils::run::{run, run_in_dir, run_with_env, run_with_input};
use utils::{empty_bin_dir, mock_bin, output_file_exists, read_output_file, remove_output_file};

// Warning: These tests MUST be run sequentially. Running them in
// parallel threads may cause conflicts with environment variables,
// as a variable may be overridden before it is used.
//
// `just test` already runs the suite with `--test-threads=1`. If we
// need parallel-safe tests later, the migration path is to allocate a
// per-test temp bin dir and thread it into process-local env setup
// instead of mutating the global env.

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
-not equal
+this is bar/baz

biz.txt
@@ -1,1 +1,1 @@
-not equal
+this is bar/baz

boz.txt
! File does not exist in home.
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
-this is bar/baz
+not equal

biz.txt
@@ -1,1 +1,1 @@
-this is bar/baz
+not equal

boz.txt
! File does not exist in home.
! Skipping...
"
    );
}

#[test]
fn diff_detects_missing_trailing_newline() {
    conf::init();

    conf::create_file_in_configs("foo.txt", Some("same content"));
    conf::create_file_in_home("foo.txt", Some("same content\n"));

    let output = run(&["diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
foo.txt
@@ -1,1 +1,1 @@
-same content
+same content
"
    );
}

#[test]
fn diff_incoming_shows_git_output_verbatim() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_incoming");

    let output = run(&["diff", "--incoming", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Git's output is passed through untouched.
    assert_eq!(
        output.stdout,
        "\
diff --git a/.gitconfig b/.gitconfig
index 1111111..2222222 100644
--- a/.gitconfig
+++ b/.gitconfig
@@ -1,2 +1,2 @@
 [user]
-	name = Old Name
+	name = New Name
"
    );

    let args = read_output_file("output_git_args");
    let mut args = args.lines();

    assert_eq!(args.next(), Some("fetch"));
    assert_eq!(args.next(), Some("diff --no-color HEAD...@{u} --"));
    assert_eq!(args.next(), None);
}

#[test]
fn diff_incoming_shortcut() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_incoming");

    let output = run(&["df", "-i", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(read_output_file("output_git_args").contains("HEAD...@{u}"));
}

/// `--reversed` swaps the endpoints, as it does for a regular `diff`.
/// For `--incoming`, that means showing outgoing changes instead.
#[test]
fn diff_incoming_reversed_shows_outgoing() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_incoming");

    let output = run(&["diff", "-i", "-r", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    let args = read_output_file("output_git_args");
    let mut args = args.lines();

    assert_eq!(args.next(), Some("fetch"));
    assert_eq!(args.next(), Some("diff --no-color @{u}...HEAD --"));
}

/// Git has no `NO_COLOR` support, it only knows about `color.ui` and
/// friends. `deez` forwards the user's preference for it.
#[test]
fn diff_incoming_forwards_no_color() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_incoming");

    // `run()` sets `NO_COLOR` for us.
    let output = run(&["diff", "-i", &conf::root()]);
    dbg!(&output.stdout);
    assert_eq!(output.exit_code, 0);
    assert!(read_output_file("output_git_args").contains("diff --no-color"));

    remove_output_file("output_git_args");

    let output = run_with_env(
        &["diff", "-i", &conf::root()],
        conf::root(),
        &[("NO_COLOR", None)],
    );
    dbg!(&output.stdout);
    assert_eq!(output.exit_code, 0);
    assert!(!read_output_file("output_git_args").contains("--no-color"));
}

#[test]
fn diff_incoming_propagates_fetch_failure() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_fetch_fails");

    conf::create_executable_file_in_configs("pre-diff.sh", None);
    conf::create_executable_file_in_configs("post-diff.sh", None);

    let output = run(&["--verbose", "diff", "-i", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 128);

    // Git explains itself; `deez` doesn't add to it.
    assert!(output.stderr.contains("fatal: unable to access"));

    // `diff` is never reached, and no hook runs.
    let args = read_output_file("output_git_args");
    assert!(!args.contains("diff"));
    assert!(!output.stdout.contains("hook: "));
}

/// Contrary to a fetch failure, a `diff` failure happens _after_
/// `pre-diff` has run.
#[test]
fn diff_incoming_propagates_diff_failure() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_diff_fails");

    conf::create_executable_file_in_configs("pre-diff.sh", None);
    conf::create_executable_file_in_configs("post-diff.sh", None);

    let output = run(&["--verbose", "diff", "-i", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 128);
    assert!(output.stderr.contains("fatal: no upstream configured"));

    assert!(output.stdout.contains("hook: pre-diff.sh"));
    assert!(!output.stdout.contains("hook: post-diff.sh"));
}

#[test]
fn diff_incoming_runs_diff_hooks() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_incoming");

    conf::create_executable_file_in_configs("pre-diff.sh", None);
    conf::create_executable_file_in_configs("post-diff.sh", None);

    let output = run(&["--verbose", "diff", "-i", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("Ran 2 hooks."));

    // Contrary to a regular `diff`, `post-diff` runs _after_ the
    // output, because Git waits for its own pager.
    let patch = output.stdout.find("@@ -1,2 +1,2 @@").unwrap();
    let post_diff = output.stdout.find("hook: post-diff.sh").unwrap();
    assert!(post_diff > patch);
}

#[test]
fn diff_incoming_rejects_pull() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_incoming");

    let output = run(&["diff", "-i", "--pull", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 2);
    assert!(
        output
            .stderr
            .contains("'--incoming' cannot be combined with '--pull'.")
    );

    // Above all, no pull happened.
    assert!(!output_file_exists("output_git_args"));
}

#[test]
fn diff_incoming_rejects_remote_root() {
    conf::init();

    remove_output_file("output_git_args");
    mock_bin("git", "bin_git_incoming");

    let output = run(&["diff", "-i", "https://github.com/qrichert/configs"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 2);
    assert!(
        output
            .stderr
            .contains("'--incoming' only works with local config roots.")
    );
    assert!(!output_file_exists("output_git_args"));
}

/// Every other error is Git's to report, so this is the only message
/// `deez` writes itself on this code path.
#[test]
fn diff_incoming_without_git_installed() {
    conf::init();

    let output = run_with_env(
        &["diff", "-i", &conf::root()],
        conf::root(),
        &[("PATH", Some(empty_bin_dir()))],
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.stderr,
        "\
fatal: Could not run 'git fetch'.
Did not find the 'git' executable. Please ensure Git is properly
installed on your machine.
"
    );
}

#[test]
fn diff_pull_runs_git_pull() {
    conf::init();

    remove_output_file("output_args");
    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["diff", "--pull", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read_output_file("output_args").trim(), "pull");
}

/// `diff` normally accepts an unmarked root without confirmation
/// because it is read-only. With `--pull`, it asks first because
/// `git pull` can modify the root.
#[test]
fn diff_pull_prompts_once_for_non_config_root() {
    conf::init();

    let root = conf::create_dir_in_configs("unmarked");
    let root = root.display().to_string();

    conf::create_file_in_configs("unmarked/foo.txt", Some("this is foo"));

    // Plain `diff` does not prompt for the unmarked root.
    let output = run(&["diff", &root]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(!output.stdout.contains("Proceed? (y/N) "));
    assert!(
        !output
            .stderr
            .contains("warning: `root` is not a configuration root.")
    );

    // With `--pull`, it asks exactly once before running `git pull`.
    remove_output_file("output_args");
    mock_bin("git", "bin_output_args_to_file");

    let output = run_with_input(&["diff", "--pull", &root], "y\n");
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout.matches("Proceed? (y/N) ").count(), 1);
    assert_eq!(
        output
            .stderr
            .matches("warning: `root` is not a configuration root.")
            .count(),
        1
    );
    assert_eq!(read_output_file("output_args").trim(), "pull");
}

#[test]
fn diff_pull_rejects_remote_root() {
    conf::init();

    remove_output_file("output_args");

    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["diff", "--pull", "https://github.com/qrichert/configs"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 2);
    assert!(
        output
            .stderr
            .contains("'--pull' only works with local config roots.")
    );
    assert!(!output_file_exists("output_args"));
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
! File does not exist in home.
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
fn diff_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);
    conf::create_file_in_home("foo", None);

    utils::create_all_command_hooks(Some("# hook"));

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
fn diff_hooks_abort_execution_if_exit_code_is_non_zero() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);

    conf::create_executable_file_in_configs("pre-diff.sh", Some(r"exit 1"));

    let output = run(&["--verbose", "diff", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    utils::assert_aborted_by(&output, "pre-diff.sh");

    // The aborted `diff` printed nothing about the configs.
    assert!(!output.stdout.contains(".gitconfig"));
}
