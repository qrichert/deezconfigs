#![allow(clippy::many_single_char_names)]

mod utils;

#[path = "utils/hook_macros.rs"]
mod hook_macros;

// Generate shared hook tests for this command.
hook_macros::hook_tests!(rsync);

use std::env;
use std::path::Path;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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

    assert_eq!(files::read(&git), "new");
    assert_eq!(files::read(&nvim), "new");
    assert_eq!(files::read(&fish), "new");
    assert_eq!(files::read(&ghostty), "new");
}

#[test]
fn rsync_with_pathspec_only_rsyncs_that_subtree() {
    conf::init();

    let git = conf::create_file_in_configs(".gitconfig", Some("old"));
    let fish = conf::create_file_in_configs(".config/fish/config.fish", Some("old"));

    conf::create_file_in_home(".gitconfig", Some("new"));
    conf::create_file_in_home(".config/fish/config.fish", Some("new"));

    let output = run(&["rsync", &conf::root(), "--", ".config/fish"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(files::read(&fish), "new"); // Updated from home.
    assert_eq!(files::read(&git), "old"); // Left untouched.
}

#[test]
fn rsync_with_negation_excludes_matching_files() {
    conf::init();

    let git = conf::create_file_in_configs(".gitconfig", Some("old"));
    let fish = conf::create_file_in_configs(".config/fish/config.fish", Some("old"));

    conf::create_file_in_home(".gitconfig", Some("new"));
    conf::create_file_in_home(".config/fish/config.fish", Some("new"));

    let output = run(&["rsync", &conf::root(), "--", ":!.config/fish/config.fish"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(files::read(&git), "new");
    assert_eq!(files::read(&fish), "old");
}

#[test]
fn rsync_with_glob_pathspecs_includes_and_excludes_matching_files() {
    conf::init();

    let root_toml = conf::create_file_in_configs("root.toml", Some("old"));
    let keep = conf::create_file_in_configs("nested/keep.toml", Some("old"));
    let skip = conf::create_file_in_configs("nested/skip.toml", Some("old"));
    let other = conf::create_file_in_configs("nested/other.txt", Some("old"));

    conf::create_file_in_home("root.toml", Some("new"));
    conf::create_file_in_home("nested/keep.toml", Some("new"));
    conf::create_file_in_home("nested/skip.toml", Some("new"));
    conf::create_file_in_home("nested/other.txt", Some("new"));

    let output = run(&["rsync", &conf::root(), "--", "**/*.toml", ":!**/skip.*"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(files::read(&root_toml), "new");
    assert_eq!(files::read(&keep), "new");
    assert_eq!(files::read(&skip), "old");
    assert_eq!(files::read(&other), "old");
}

#[test]
fn rsync_with_invalid_pathspec_errors_and_rsyncs_nothing() {
    conf::init();

    let git = conf::create_file_in_configs(".gitconfig", Some("old"));
    conf::create_file_in_home(".gitconfig", Some("new"));

    let output = run(&["rsync", &conf::root(), "--", ".."]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("Invalid pathspec"));
    assert_eq!(files::read(&git), "old");
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

    assert_eq!(
        output.stdout,
        "\
rSynced 4 files.
Ran 2 hooks.
"
    );
}

#[test]
fn rsync_does_not_report_updated_file_count_without_verbose() {
    conf::init();

    conf::create_file_in_configs("unchanged.txt", Some("same"));
    conf::create_file_in_configs("modified.txt", Some("old"));
    conf::create_file_in_configs("missing.txt", Some("untouched"));

    conf::create_file_in_home("unchanged.txt", Some("same"));
    conf::create_file_in_home("modified.txt", Some("new"));

    let output = run(&["rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(!output.stdout.contains("Updated"));
    assert_eq!(files::read_in_configs("unchanged.txt"), "same");
    assert_eq!(files::read_in_configs("modified.txt"), "new");
    assert_eq!(files::read_in_configs("missing.txt"), "untouched");
}

#[test]
fn rsync_verbose_reports_updated_file_count() {
    conf::init();

    conf::create_file_in_configs("unchanged.txt", Some("same"));
    conf::create_file_in_configs("modified.txt", Some("old"));
    conf::create_file_in_configs("missing.txt", Some("untouched"));

    conf::create_file_in_home("unchanged.txt", Some("same"));
    conf::create_file_in_home("modified.txt", Some("new"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "\
missing.txt
modified.txt
unchanged.txt
rSynced 3 files. Updated 1.
"
    );
    assert_eq!(files::read_in_configs("unchanged.txt"), "same");
    assert_eq!(files::read_in_configs("modified.txt"), "new");
    assert_eq!(files::read_in_configs("missing.txt"), "untouched");
}

#[cfg(unix)]
#[test]
fn rsync_verbose_counts_permission_only_changes_as_updates() {
    conf::init();

    let destination = conf::create_file_in_configs("script.sh", Some("same"));
    let source = conf::create_file_in_home("script.sh", Some("same"));
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o644)).unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();

    let output = run(&["--verbose", "rsync", &conf::root(), "--", "script.sh"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "\
script.sh
rSynced 1 file. Updated 1.
"
    );
    assert_eq!(
        fs::metadata(destination).unwrap().permissions().mode() & 0o7777,
        0o755
    );
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

    assert_eq!(
        output.stdout,
        "\
hook: pre-rsync.sh
.config/fish/config.fish
.config/ghostty/config
.config/nvim/init.lua
.gitconfig
hook: post-rsync.sh
rSynced 4 files. Updated 0.
Ran 2 hooks.
"
    );
}

#[test]
fn rsync_pull_runs_git_pull() {
    conf::init();

    remove_output_file("output_args");
    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["rsync", "--pull", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read_output_file("output_args").trim(), "pull");
}

/// `rsync` only accepts local roots. This checks that `--pull` reports a
/// clear error for a remote URI instead of treating it as a local path.
#[test]
fn rsync_pull_rejects_remote_root() {
    conf::init();

    remove_output_file("output_args");

    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["rsync", "--pull", "https://github.com/qrichert/configs"]);
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
fn rsync_ignores_special_files() {
    conf::init();

    // OK.
    let sub_gitconfig = conf::create_file_in_configs("subdir/.git/config", Some("old"));
    // NOT OK.
    let ignore = conf::create_file_in_configs(".ignore", Some("old"));
    let sub_ignore = conf::create_file_in_configs("subdir/.ignore", Some("old"));
    let gitignore = conf::create_file_in_configs(".gitignore", Some("old"));
    let gitconfig = conf::create_file_in_configs(".git/config", Some("old"));
    let sub_gitignore = conf::create_file_in_configs("subdir/.gitignore", Some("old"));
    let sub_deez = conf::create_file_in_configs("subdir/.deez", Some("old"));

    // OK.
    conf::create_file_in_home("subdir/.git/config", Some("new"));
    // NOT OK.
    conf::create_file_in_home(".ignore", Some("new"));
    conf::create_file_in_home("subdir/.ignore", Some("new"));
    conf::create_file_in_home(".gitignore", Some("new"));
    conf::create_file_in_home(".git/config", Some("new"));
    conf::create_file_in_home("subdir/.gitignore", Some("new"));
    conf::create_file_in_home("subdir/.deez", Some("new"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK.
    assert_eq!(files::read(&sub_gitconfig), "new");
    // NOT OK.
    assert_eq!(files::read(&ignore), "old");
    assert_eq!(files::read(&sub_ignore), "old");
    assert_eq!(files::read(&gitignore), "old");
    assert_eq!(files::read(&gitconfig), "old");
    assert_eq!(files::read(&sub_gitignore), "old");
    assert_eq!(files::read(&sub_deez), "old");
}

/// If we have a `.vimrc` symlink pointing at the `vimrc.vim` in
/// `.config`, we don't want it to be replaced with a file, but to
/// update the target content.
#[test]
fn rsync_does_not_replace_symlink_with_file() {
    conf::init();

    // Real file in home.
    conf::create_file_in_home("config_file.txt", Some("new content"));

    // Target file that should be overridden.
    let symlink_target_in_configs =
        conf::create_file_in_configs("symlink_target.txt", Some("should be replaced"));
    conf::create_file_in_configs(".ignore", Some("symlink_target.txt"));

    // Symlink in configs.
    let (symlink_in_configs, _) =
        conf::create_symlink_in_configs("config_file.txt", Some("symlink_target.txt"));

    // Ensure the symlink in configs links to target file.
    assert!(symlink_in_configs.is_symlink());
    assert_eq!(files::read(&symlink_in_configs), "should be replaced");

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Ensure the symlink in configs still is a symlink.
    assert!(symlink_in_configs.is_symlink());
    assert_eq!(files::read(&symlink_in_configs), "new content");

    // Ensure the symlink's target has been updated.
    assert_eq!(files::read(&symlink_target_in_configs), "new content");
}

/// If a symlink in home links to a file in configs, copying it back to
/// configs (i.e, `cp B A` where `B@ -> A`) would (likely) truncate the
/// file. This behaviour is documented in `std::fs::copy()` (Rust 1.86)
/// and observed at least on macOS. This should be a no-op for us since
/// a symlink is always up-to-date.
#[test]
fn rsync_errors_if_symlink_in_home_links_to_file_in_configs() {
    conf::init();

    let file = conf::create_file_in_configs("a.txt", Some("Hello from A!"));
    conf::create_symlink_in_home("a.txt", Some(&file.to_string_lossy()));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Look for a no-op:

    assert!(files::file_exists_in_configs("a.txt"));
    assert_eq!(files::read_in_configs("a.txt"), "Hello from A!");

    assert!(files::symlink_exists_in_home("a.txt"));
    assert_eq!(files::read_in_home("a.txt"), "Hello from A!");
    dbg!(files::read_symlink_in_home("a.txt"));
    assert_eq!(files::read_symlink_in_home("a.txt"), file);
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

    assert_eq!(files::read(&a), "old");
    assert_eq!(files::read(&b), "old");
    assert_eq!(files::read(&c), "new");

    assert_ne!(files::read(&ignore), "new");
    assert_ne!(files::read(&gitignore), "new");
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

    assert_eq!(files::read(&file), "new");
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

    assert_eq!(files::read(&file), "new");
}

#[test]
fn rsync_uses_deez_root_variable_if_no_root_specified() {
    conf::init();

    let file = conf::create_file_in_configs("bar.txt", Some("old"));

    conf::create_file_in_home("bar.txt", Some("new"));

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    // Run outside of any root. It should use `DEEZ_ROOT`.
    let output = run_in_dir(
        &["--verbose", "rsync"],
        Path::new(&conf::root()).parent().unwrap(),
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(files::read(&file), "new");
}

#[test]
fn rsync_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);

    // Config-side hooks, and stale same-named files in the home.
    utils::create_all_command_hooks(Some("# old"));
    utils::create_all_command_hooks_in_home(Some("new"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // `rsync` must not pull the home versions over the config hooks.
    utils::assert_config_hooks_unchanged("# old");
}

#[test]
fn rsync_hooks_are_not_copied_from_home() {
    conf::init();

    // Regular files (configs).
    let sub_pre = conf::create_file_in_configs("foo/pre-rsync.sh", Some("old"));
    let sub_post = conf::create_file_in_configs("foo/post-rsync.sh", Some("old"));

    // Hooks (configs).
    let pre = conf::create_executable_file_in_configs("pre-rsync.sh", Some("# old"));
    let post = conf::create_executable_file_in_configs("post-rsync.sh", Some("# old"));

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
    assert_eq!(files::read(&sub_pre), "new");
    assert_eq!(files::read(&sub_post), "new");

    // Hooks are not copied.
    assert_eq!(files::read(&pre), "# old");
    assert_eq!(files::read(&post), "# old");
}

#[test]
fn rsync_hooks_abort_execution_if_exit_code_is_non_zero() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", Some("old"));
    conf::create_file_in_home(".gitconfig", Some("new"));

    conf::create_executable_file_in_configs("pre-rsync.sh", Some(r"exit 1"));

    let output = run(&["--verbose", "rsync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    utils::assert_aborted_by(&output, "pre-rsync.sh");

    // The aborted `rsync` did not pull the home version into configs.
    assert_eq!(files::read_in_configs(".gitconfig"), "old");
}
