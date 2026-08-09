mod utils;

#[path = "utils/hook_macros.rs"]
mod hook_macros;

// Generate shared hook tests for this command.
hook_macros::hook_tests!(clean);

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

    assert!(!files::file_exists_in_home(".gitconfig"));
    assert!(!files::file_exists_in_home(".config/nvim/init.lua"));
    assert!(!files::file_exists_in_home(".config/fish/config.fish"));
    assert!(!files::file_exists_in_home(".config/ghostty/config"));
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
fn clean_pull_runs_git_pull() {
    conf::init();

    remove_output_file("output_args");
    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["clean", "--pull", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read_output_file("output_args").trim(), "pull");
}

#[test]
fn clean_pull_rejects_remote_root() {
    conf::init();

    remove_output_file("output_args");

    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["clean", "--pull", "https://github.com/qrichert/configs"]);
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
fn clean_cleans_up_remote_clone() {
    conf::init();

    remove_output_file("output_clone_path");
    mock_bin("git", "bin_git_clone");

    let output = run(&["--verbose", "clean", "git:success[sub/root]"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    let clone_path = read_output_file("output_clone_path");
    assert!(!Path::new(clone_path.trim()).exists());
}

#[test]
fn clean_cleans_up_partial_clone_after_git_failure() {
    conf::init();

    remove_output_file("output_clone_path");
    mock_bin("git", "bin_git_clone");

    let output = run(&["--verbose", "clean", "git:fail"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 1);

    let clone_path = read_output_file("output_clone_path");
    assert!(!Path::new(clone_path.trim()).exists());
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
    assert!(!files::dir_exists_in_home("deeply/nested/"));
    // `deeply/` still contains a file and should _not_ be deleted.
    assert!(files::dir_exists_in_home("deeply/"));
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
    assert!(!files::file_exists_in_home("deeply/nested/file/foo.txt"));

    // Home itself still exists.
    assert!(files::dir_exists_in_home("./"));
}

#[test]
fn clean_ignores_special_files() {
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

    // OK.
    conf::create_file_in_home("subdir/.git/config", None);
    // NOT OK.
    conf::create_file_in_home(".ignore", None);
    conf::create_file_in_home("subdir/.ignore", None);
    conf::create_file_in_home(".gitignore", None);
    conf::create_file_in_home(".git/config", None);
    conf::create_file_in_home("subdir/.gitignore", None);
    conf::create_file_in_home("subdir/.deez", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK.
    assert!(!files::file_exists_in_home("subdir/.git/config"));
    // NOT OK.
    assert!(files::file_exists_in_home(".ignore"));
    assert!(files::file_exists_in_home("subdir/.ignore"));
    assert!(files::file_exists_in_home(".gitignore"));
    assert!(files::file_exists_in_home(".git/config"));
    assert!(files::file_exists_in_home("subdir/.gitignore"));
    assert!(files::file_exists_in_home("subdir/.deez"));
}

#[test]
fn clean_replaces_existing_directory_if_empty() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    conf::create_dir_in_home("foo.txt");

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!files::dir_exists_in_home("foo.txt"));
}

#[test]
fn clean_replaces_existing_directory_only_if_empty() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    // `foo.txt` directory is not empty.
    conf::create_file_in_home("foo.txt/baz.log", None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 1);

    assert!(files::dir_exists_in_home("foo.txt"));
    assert!(files::file_exists_in_home("foo.txt/baz.log"));
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

    assert!(files::file_exists_in_home("foo/a.txt"));
    assert!(files::file_exists_in_home("bar/b.txt"));
    assert!(!files::file_exists_in_home("baz/c.txt"));

    assert!(files::file_exists_in_home(".ignore"));
    assert!(files::file_exists_in_home(".gitignore"));
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

    assert!(!files::file_exists_in_home("foo/bar/baz.txt"));
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

    assert!(!files::file_exists_in_home("foo/bar.txt"));
}

#[test]
fn clean_uses_deez_root_variable_if_no_root_specified() {
    conf::init();

    conf::create_file_in_configs("bar.txt", None);
    conf::create_file_in_home("bar.txt", None);

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    // Run outside of any root. It should use `DEEZ_ROOT`.
    let output = run_in_dir(
        &["--verbose", "clean"],
        Path::new(&conf::root()).parent().unwrap(),
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!files::file_exists_in_home("bar.txt"));
}

#[test]
fn clean_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);

    // Hooks in configs, and same-named regular files already in the home.
    utils::create_all_command_hooks(None);
    utils::create_all_command_hooks_in_home(None);

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // `clean` removes tracked configs from the home, but hook files are
    // not configs, so the home copies survive.
    utils::assert_all_command_hooks_survive_in_home();
}

#[test]
fn clean_hooks_abort_execution_if_exit_code_is_non_zero() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_home(".gitconfig", None);

    conf::create_executable_file_in_configs("pre-clean.sh", Some(r"exit 1"));

    let output = run(&["--verbose", "clean", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    utils::assert_aborted_by(&output, "pre-clean.sh");

    // The aborted `clean` did not delete anything from the home.
    assert!(files::file_exists_in_home(".gitconfig"));
}
