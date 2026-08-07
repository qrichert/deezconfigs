mod utils;

#[path = "utils/hook_macros.rs"]
mod hook_macros;

// Generate shared hook tests for this command.
hook_macros::hook_tests!(status);

use std::env;
use std::path::Path;

use utils::conf;
use utils::run::{run, run_in_dir, run_with_input};
use utils::{mock_bin, output_file_exists, read_output_file, remove_output_file};

// Warning: These tests MUST be run sequentially. Running them in
// parallel threads may cause conflicts with environment variables,
// as a variable may be overridden before it is used.
//
// `just test` already runs the suite with `--test-threads=1`. If we
// need parallel-safe tests later, the migration path is to allocate a
// per-test temp bin dir and thread it into process-local env setup
// instead of mutating the global env.

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
2 in sync, 2 modified, 1 missing.
"
    );
}

#[test]
fn status_pull_runs_git_pull() {
    conf::init();

    remove_output_file("output_args");
    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["status", "--pull", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read_output_file("output_args").trim(), "pull");
}

/// `status` normally accepts an unmarked root without confirmation
/// because it is read-only. With `--pull`, it asks first because
/// `git pull` can modify the root.
#[test]
fn status_pull_prompts_once_for_non_config_root() {
    conf::init();

    let root = conf::create_dir_in_configs("unmarked");
    let root = root.display().to_string();

    conf::create_file_in_configs("unmarked/foo.txt", Some("this is foo"));

    // Plain `status` does not prompt for the unmarked root.
    let output = run(&["status", &root]);
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

    let output = run_with_input(&["status", "--pull", &root], "y\n");
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
fn status_pull_rejects_remote_root() {
    conf::init();

    remove_output_file("output_args");

    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["status", "--pull", "https://github.com/qrichert/configs"]);
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
0 in sync, 0 modified, 1 missing.
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
    // NOT OK.
    conf::create_file_in_configs(".ignore", None);
    conf::create_file_in_configs("subdir/.ignore", None);
    conf::create_file_in_configs(".gitignore", None);
    conf::create_file_in_configs(".git/config", None);
    conf::create_file_in_configs("subdir/.gitignore", None);
    conf::create_file_in_configs("subdir/.deez", None);

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK.
    assert!(output.stdout.contains("subdir/.git/config"));
    // NOT OK.
    assert!(!output.stdout.contains(" .ignore"));
    assert!(!output.stdout.contains("subdir/.ignore"));
    assert!(!output.stdout.contains(" .gitignore"));
    assert!(!output.stdout.contains(" .git/config"));
    assert!(!output.stdout.contains("subdir/.gitignore"));
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
fn status_uses_deez_root_variable_if_no_root_specified() {
    conf::init();

    conf::create_file_in_configs("bar.txt", None);

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    // Run outside of any root. It should use `DEEZ_ROOT`.
    let output = run_in_dir(
        &["--verbose", "status"],
        Path::new(&conf::root()).parent().unwrap(),
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains("bar.txt"));
}

#[test]
fn status_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);

    utils::create_all_command_hooks(None);

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Hooks appear under the `Hooks` listing, not as config files.
    utils::assert_hooks_section_listed(&output);
}

#[test]
fn status_hooks_abort_execution_if_exit_code_is_non_zero() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);

    conf::create_executable_file_in_configs("pre-status.sh", Some(r"exit 1"));

    let output = run(&["--verbose", "status", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    utils::assert_aborted_by(&output, "pre-status.sh");

    // The aborted `status` printed nothing about the configs.
    assert!(!output.stdout.contains(".gitconfig"));
}
