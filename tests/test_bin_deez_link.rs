mod utils;

#[path = "utils/hook_macros.rs"]
mod hook_macros;

// Generate shared hook tests for this command.
hook_macros::hook_tests!(link);

use std::env;
use std::path::Path;

use utils::conf;
use utils::files;
use utils::run::{run, run_in_dir};
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

    assert!(files::symlink_exists_in_home(".gitconfig"));
    assert!(files::symlink_exists_in_home(".config/nvim/init.lua"));
    assert!(files::symlink_exists_in_home(".config/fish/config.fish"));
    assert!(files::symlink_exists_in_home(".config/ghostty/config"));
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

    assert_eq!(files::read_in_home("foo.txt"), "this is foo");
    assert_eq!(files::read_in_home("bar/baz.txt"), "this is bar/baz");
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

    assert_eq!(
        output.stdout,
        "\
Linked 4 files.
Ran 2 hooks.
"
    );
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

    assert_eq!(
        output.stdout,
        "\
hook: pre-link.sh
.config/fish/config.fish
.config/ghostty/config
.config/nvim/init.lua
.gitconfig
hook: post-link.sh
Linked 4 files.
Ran 2 hooks.
"
    );
}

#[test]
fn link_pull_runs_git_pull() {
    conf::init();

    remove_output_file("output_args");
    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["link", "--pull", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read_output_file("output_args").trim(), "pull");
}

/// `link` only accepts local roots. This checks that `--pull` reports a
/// clear error for a remote URI instead of treating it as a local path.
#[test]
fn link_pull_rejects_remote_root() {
    conf::init();

    remove_output_file("output_args");

    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["link", "--pull", "https://github.com/qrichert/configs"]);
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
fn link_ignores_special_files() {
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

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK.
    assert!(files::symlink_exists_in_home("subdir/.git/config"));
    // NOT OK.
    assert!(!files::symlink_exists_in_home(".ignore"));
    assert!(!files::symlink_exists_in_home("subdir/.ignore"));
    assert!(!files::symlink_exists_in_home(".gitignore"));
    assert!(!files::symlink_exists_in_home(".git/config"));
    assert!(!files::symlink_exists_in_home("subdir/.gitignore"));
    assert!(!files::symlink_exists_in_home("subdir/.deez"));
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
    assert_eq!(files::read(&file_in_home), "new");

    // Ensure the symlink in home points to the updated target.
    assert!(symlink_in_home.is_symlink());
    assert_eq!(files::read(&symlink_in_home), "new");
}

#[test]
fn link_replaces_existing_directory_if_empty() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    conf::create_dir_in_home("foo.txt");

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!files::dir_exists_in_home("foo.txt"));
    assert!(files::symlink_exists_in_home("foo.txt"));
}

#[test]
fn link_replaces_existing_directory_only_if_empty() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    // `foo.txt` directory is not empty.
    conf::create_file_in_home("foo.txt/baz.log", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 1);

    assert!(files::dir_exists_in_home("foo.txt"));
    assert!(files::file_exists_in_home("foo.txt/baz.log"));
    assert!(!files::symlink_exists_in_home("foo.txt"));
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

    assert!(!files::symlink_exists_in_home("foo/a.txt"));
    assert!(!files::symlink_exists_in_home("bar/b.txt"));
    assert!(files::symlink_exists_in_home("baz/c.txt"));

    assert!(!files::symlink_exists_in_home(".ignore"));
    assert!(!files::symlink_exists_in_home(".gitignore"));
}

#[test]
fn link_looks_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", None);

    let output = run_in_dir(&["--verbose", "link"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::symlink_exists_in_home("foo/bar/baz.txt"));
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

    assert!(files::symlink_exists_in_home("foo/bar.txt"));
}

#[test]
fn link_uses_deez_root_variable_if_no_root_specified() {
    conf::init();

    conf::create_file_in_configs("bar.txt", None);

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    // Run outside of any root. It should use `DEEZ_ROOT`.
    let output = run_in_dir(
        &["--verbose", "link"],
        Path::new(&conf::root()).parent().unwrap(),
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::symlink_exists_in_home("bar.txt"));
}

#[test]
fn link_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);

    utils::create_all_command_hooks(None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Hooks are never linked into the home.
    utils::assert_no_command_hooks_in_home();
}

#[test]
fn link_hooks_are_not_copied_to_home() {
    conf::init();

    // Regular files.
    conf::create_file_in_configs("foo/pre-link.sh", None);
    conf::create_file_in_configs("foo/post-link.sh", None);

    // Hooks.
    conf::create_executable_file_in_configs("pre-link.sh", None);
    conf::create_executable_file_in_configs("post-link.sh", None);

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Non-root "hooks" are not hooks, but regular files.
    assert!(files::symlink_exists_in_home("foo/pre-link.sh"));
    assert!(files::symlink_exists_in_home("foo/post-link.sh"));

    // Hooks are not copied.
    assert!(!files::file_exists_in_home("pre-link.sh"));
    assert!(!files::file_exists_in_home("post-link.sh"));
}

#[test]
fn link_hooks_abort_execution_if_exit_code_is_non_zero() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);

    conf::create_executable_file_in_configs("pre-link.sh", Some(r"exit 1"));

    let output = run(&["--verbose", "link", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    utils::assert_aborted_by(&output, "pre-link.sh");

    // The aborted `link` did not symlink anything into the home.
    assert!(!files::symlink_exists_in_home(".gitconfig"));
}
