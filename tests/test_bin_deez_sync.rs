mod utils;

#[path = "utils/hook_macros.rs"]
mod hook_macros;

use std::env;
use std::path::{Path, PathBuf};

use utils::conf::{self, CONFIGS};
use utils::files;
use utils::run::{run, run_in_dir};
use utils::{mock_bin, output_file_exists, read_output_file, remove_output_file};

// Generate shared hook tests for this command.
hook_macros::hook_tests!(sync);

// Warning: These tests MUST be run sequentially. Running them in
// parallel threads may cause conflicts with environment variables,
// as a variable may be overridden before it is used.
//
// `just test` already runs the suite with `--test-threads=1`. If we
// need parallel-safe tests later, the migration path is to allocate a
// per-test temp bin dir and thread it into process-local env setup
// instead of mutating the global env.

#[test]
fn sync_regular() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::file_exists_in_home(".gitconfig"));
    assert!(files::file_exists_in_home(".config/nvim/init.lua"));
    assert!(files::file_exists_in_home(".config/fish/config.fish"));
    assert!(files::symlink_exists_in_home(".config/ghostty/config"));
}

#[test]
fn sync_with_pathspec_only_syncs_that_subtree() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);

    // Only the `.config/fish` subtree is selected.
    let output = run(&["sync", &conf::root(), "--", ".config/fish"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::file_exists_in_home(".config/fish/config.fish"));
    assert!(!files::file_exists_in_home(".gitconfig"));
    assert!(!files::file_exists_in_home(".config/nvim/init.lua"));
}

#[test]
fn sync_with_negation_excludes_matching_files() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);

    let output = run(&["sync", &conf::root(), "--", ":!.config/fish/config.fish"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!files::file_exists_in_home(".config/fish/config.fish"));
    assert!(files::file_exists_in_home(".gitconfig"));
    assert!(files::file_exists_in_home(".config/nvim/init.lua"));
}

#[test]
fn sync_with_glob_pathspecs_includes_and_excludes_matching_files() {
    conf::init();

    conf::create_file_in_configs("root.toml", None);
    conf::create_file_in_configs("nested/keep.toml", None);
    conf::create_file_in_configs("nested/skip.toml", None);
    conf::create_file_in_configs("nested/other.txt", None);

    let output = run(&["sync", &conf::root(), "--", "**/*.toml", ":!**/skip.*"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(files::file_exists_in_home("root.toml"));
    assert!(files::file_exists_in_home("nested/keep.toml"));
    assert!(!files::file_exists_in_home("nested/skip.toml"));
    assert!(!files::file_exists_in_home("nested/other.txt"));
}

#[test]
fn sync_with_invalid_pathspec_errors_and_syncs_nothing() {
    // /!\ An invalid pathspec must fail closed, NEVER fall back to
    // removing everything.
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);

    let output = run(&["sync", &conf::root(), "--", ".."]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("Invalid pathspec"));
    assert!(!files::file_exists_in_home(".gitconfig"));
}

#[test]
fn sync_output() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
Synced 4 files.
Ran 2 hooks.
"
    );
}

#[test]
fn sync_does_not_report_updated_file_count_without_verbose() {
    conf::init();

    conf::create_file_in_configs("unchanged.txt", Some("same"));
    conf::create_file_in_configs("modified.txt", Some("new"));
    conf::create_file_in_configs("missing.txt", Some("created"));

    conf::create_file_in_home("unchanged.txt", Some("same"));
    conf::create_file_in_home("modified.txt", Some("old"));

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(!output.stdout.contains("Updated"));
    assert_eq!(files::read_in_home("unchanged.txt"), "same");
    assert_eq!(files::read_in_home("modified.txt"), "new");
    assert_eq!(files::read_in_home("missing.txt"), "created");
}

#[test]
fn sync_verbose_reports_updated_file_count() {
    conf::init();

    conf::create_file_in_configs("unchanged.txt", Some("same"));
    conf::create_file_in_configs("modified.txt", Some("new"));
    conf::create_file_in_configs("missing.txt", Some("created"));

    conf::create_file_in_home("unchanged.txt", Some("same"));
    conf::create_file_in_home("modified.txt", Some("old"));

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "\
missing.txt
modified.txt
unchanged.txt
Synced 3 files. Updated 2.
"
    );
    assert_eq!(files::read_in_home("unchanged.txt"), "same");
    assert_eq!(files::read_in_home("modified.txt"), "new");
    assert_eq!(files::read_in_home("missing.txt"), "created");
}

#[test]
fn sync_verbose_counts_permission_only_changes_as_updates() {
    conf::init();

    let source = conf::create_file_in_configs("script.sh", Some("same"));
    let destination = conf::create_file_in_home("script.sh", Some("same"));
    files::make_permissions_differ(&source, &destination);

    let output = run(&["--verbose", "sync", &conf::root(), "--", "script.sh"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    let permissions_were_copied = files::have_equal_permissions(&source, &destination);
    #[cfg(windows)]
    files::make_writable(&[&source, &destination]);

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "\
script.sh
Synced 1 file. Updated 1.
"
    );
    assert!(permissions_were_copied);
}

#[cfg(unix)]
#[test]
fn sync_permission_updates_with_set_id_bits_converge() {
    conf::init();

    let source = conf::create_file_in_configs("script.sh", Some("same"));
    let destination = conf::create_file_in_home("script.sh", Some("same"));
    files::set_modes(&source, &destination, 0o4755, 0o755);

    let first_output = run(&["--verbose", "sync", &conf::root(), "--", "script.sh"]);
    dbg!(&first_output.stdout);
    dbg!(&first_output.stderr);
    let mode_after_first_sync = files::mode(&destination);

    let second_output = run(&["--verbose", "sync", &conf::root(), "--", "script.sh"]);
    dbg!(&second_output.stdout);
    dbg!(&second_output.stderr);

    assert_eq!(first_output.exit_code, 0);
    assert_eq!(
        first_output.stdout,
        "script.sh\nSynced 1 file. Updated 1.\n"
    );
    assert_eq!(mode_after_first_sync, 0o4755);
    assert_eq!(second_output.exit_code, 0);
    assert_eq!(
        second_output.stdout,
        "script.sh\nSynced 1 file. Updated 0.\n"
    );
    assert_eq!(files::mode(&destination), 0o4755);
}

#[test]
fn sync_verbose_counts_file_kind_changes_as_updates() {
    conf::init();

    let symlink_target = conf::create_file_in_configs("targets/symlink-target.txt", Some("same"));
    conf::create_symlink_in_configs("symlink.conf", Some(&symlink_target.to_string_lossy()));
    conf::create_file_in_home("symlink.conf", Some("same"));

    conf::create_file_in_configs("file.conf", Some("same"));
    conf::create_file_in_home("targets/file-target.txt", Some("same"));
    conf::create_symlink_in_home("file.conf", Some("targets/file-target.txt"));

    let output = run(&[
        "--verbose",
        "sync",
        &conf::root(),
        "--",
        "symlink.conf",
        "file.conf",
    ]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "\
file.conf
symlink.conf
Synced 2 files. Updated 2.
"
    );
    assert!(!PathBuf::from(conf::HOME).join("file.conf").is_symlink());
    assert_eq!(files::read_symlink_in_home("symlink.conf"), symlink_target);
}

#[test]
fn sync_verbose_does_not_count_unchanged_symlink_target_as_updated() {
    conf::init();

    conf::create_file_in_configs("targets/target.txt", Some("configs"));
    conf::create_file_in_home("targets/target.txt", Some("home"));
    create_file_symlink(
        "targets/target.txt",
        PathBuf::from(CONFIGS).join("config.conf"),
    );
    create_file_symlink(
        "targets/target.txt",
        PathBuf::from(conf::HOME).join("config.conf"),
    );

    let output = run(&["--verbose", "sync", &conf::root(), "--", "config.conf"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "\
config.conf
Synced 1 file. Updated 0.
"
    );
    assert_eq!(
        files::read_symlink_in_home("config.conf"),
        PathBuf::from("targets/target.txt")
    );
}

#[test]
fn sync_verbose_counts_changed_symlink_target_as_updated() {
    conf::init();

    conf::create_file_in_configs("targets/config-target.txt", Some("same"));
    conf::create_file_in_home("targets/home-target.txt", Some("same"));
    create_file_symlink(
        "targets/config-target.txt",
        PathBuf::from(CONFIGS).join("config.conf"),
    );
    create_file_symlink(
        "targets/home-target.txt",
        PathBuf::from(conf::HOME).join("config.conf"),
    );

    let output = run(&["--verbose", "sync", &conf::root(), "--", "config.conf"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        output.stdout,
        "\
config.conf
Synced 1 file. Updated 1.
"
    );
    assert_eq!(
        files::read_symlink_in_home("config.conf"),
        PathBuf::from("targets/config-target.txt")
    );
}

#[test]
fn sync_output_verbose() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);
    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(
        output.stdout,
        "\
hook: pre-sync.sh
.config/fish/config.fish
.config/ghostty/config
.config/nvim/init.lua
.gitconfig
hook: post-sync.sh
Synced 4 files. Updated 4.
Ran 2 hooks.
"
    );
}

#[test]
fn sync_pull_runs_git_pull() {
    conf::init();

    remove_output_file("output_args");
    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["sync", "--pull", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert_eq!(read_output_file("output_args").trim(), "pull");
}

#[test]
fn sync_pull_rejects_remote_root() {
    conf::init();

    remove_output_file("output_args");

    mock_bin("git", "bin_output_args_to_file");

    let output = run(&["sync", "--pull", "https://github.com/qrichert/configs"]);
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
fn sync_cleans_up_remote_clone() {
    conf::init();

    remove_output_file("output_clone_path");
    mock_bin("git", "bin_git_clone");

    let output = run(&["--verbose", "sync", "git:success[sub/root]"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    let clone_path = read_output_file("output_clone_path");
    assert!(!Path::new(clone_path.trim()).exists());
}

#[test]
fn sync_cleans_up_partial_clone_after_git_failure() {
    conf::init();

    remove_output_file("output_clone_path");
    mock_bin("git", "bin_git_clone");

    let output = run(&["--verbose", "sync", "git:fail"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 1);

    let clone_path = read_output_file("output_clone_path");
    assert!(!Path::new(clone_path.trim()).exists());
}

#[test]
fn sync_ignores_special_files() {
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

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK.
    assert!(files::file_exists_in_home("subdir/.git/config"));
    // NOT OK.
    assert!(!files::file_exists_in_home(".ignore"));
    assert!(!files::file_exists_in_home("subdir/.ignore"));
    assert!(!files::file_exists_in_home(".gitignore"));
    assert!(!files::file_exists_in_home(".git/config"));
    assert!(!files::file_exists_in_home("subdir/.gitignore"));
    assert!(!files::file_exists_in_home("subdir/.deez"));
}

/// If a file in configs should override a symlink in home, ensure
/// `sync` replaces the symlink with a file, and does _not_ replace the
/// content of the target of the symlink.
#[test]
fn sync_replaces_symlink_with_file() {
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
    assert!(symlink_in_home.is_symlink());
    assert_eq!(files::read(&symlink_in_home), "should not be replaced");

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Ensure the symlink in home is a file now, with updated content.
    assert!(!symlink_in_home.is_symlink()); // `is_file()` traverses.
    assert_eq!(files::read(&symlink_in_home), "new content");

    // Ensure the removed symlink's target has not been altered.
    assert_eq!(
        files::read(&symlink_target_in_home),
        "should not be replaced"
    );
}

/// Symlinks in configs should override files in home, and match the
/// original symlink exactly (not adapt the path, nor make it a file).
#[test]
fn sync_replaces_file_with_symlink() {
    conf::init();

    conf::create_file_in_configs("foo/symlink_target.txt", Some("hello, world"));

    // `create_symlink_in_configs()` will make the symlink's target
    // absolute, but this is not what we want here, because we want to
    // test that _relative_ path stay _relative_.
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "foo/symlink_target.txt",
        PathBuf::from(CONFIGS).join("config.conf"),
    )
    .unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(
        "foo/symlink_target.txt",
        PathBuf::from(CONFIGS).join("config.conf"),
    )
    .unwrap();

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::symlink_exists_in_home("config.conf"));
    assert_eq!(files::read_in_home("config.conf"), "hello, world");
    // Same relative path as the original.
    assert_eq!(
        files::read_symlink_in_home("config.conf").to_string_lossy(),
        "foo/symlink_target.txt"
    );
}

/// By default, symlinks fail to replace anything, unless you explicitly
/// delete the target beforehand.
#[test]
fn sync_replaces_existing_file_with_symlink() {
    conf::init();

    conf::create_symlink_in_configs("config.conf", None);
    conf::create_file_in_home("config.conf", Some("ola"));

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::symlink_exists_in_home("config.conf"));
}

#[test]
fn sync_replaces_existing_directory_if_empty() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    conf::create_dir_in_home("foo.txt");

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!files::dir_exists_in_home("foo.txt"));
    assert!(files::file_exists_in_home("foo.txt"));
}

#[test]
fn sync_replaces_existing_directory_only_if_empty() {
    conf::init();

    conf::create_file_in_configs("foo.txt", None);

    // `foo.txt` directory is not empty.
    conf::create_file_in_home("foo.txt/baz.log", None);

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 1);

    assert!(files::dir_exists_in_home("foo.txt"));
    assert!(files::file_exists_in_home("foo.txt/baz.log"));
    assert!(!files::file_exists_in_home("foo.txt"));
}

#[test]
fn sync_respects_ignore_patters() {
    conf::init();

    conf::create_file_in_configs("foo/a.txt", None);
    conf::create_file_in_configs("bar/b.txt", None);
    conf::create_file_in_configs("baz/c.txt", None);

    conf::create_file_in_configs(".ignore", Some("foo/*"));
    conf::create_file_in_configs(".gitignore", Some("bar/b.txt"));

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(!files::file_exists_in_home("foo/a.txt"));
    assert!(!files::file_exists_in_home("bar/b.txt"));
    assert!(files::file_exists_in_home("baz/c.txt"));

    assert!(!files::file_exists_in_home(".ignore"));
    assert!(!files::file_exists_in_home(".gitignore"));
}

#[test]
fn sync_looks_for_root_in_parents() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar/baz.txt", None);

    let output = run_in_dir(&["--verbose", "sync"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::file_exists_in_home("foo/bar/baz.txt"));
}

/// This test is important because the implementation `skip()`s the
/// current dir (if we're looking in parents, _we know_ the current dir
/// isn't a root). This test ensures we're not skipping too far.
#[test]
fn sync_looks_for_root_in_direct_parent() {
    conf::init();

    let file = conf::create_file_in_configs("foo/bar.txt", None);

    let output = run_in_dir(&["--verbose", "sync"], file.parent().unwrap());
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::file_exists_in_home("foo/bar.txt"));
}

#[test]
fn sync_uses_deez_root_variable_if_no_root_specified() {
    conf::init();

    conf::create_file_in_configs("bar.txt", None);

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    // Run outside of any root. It should use `DEEZ_ROOT`.
    let output = run_in_dir(
        &["--verbose", "sync"],
        Path::new(&conf::root()).parent().unwrap(),
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(files::file_exists_in_home("bar.txt"));
}

#[test]
fn sync_hooks_are_not_treated_as_config_files() {
    conf::init();

    conf::create_file_in_configs("foo", None);

    utils::create_all_command_hooks(None);

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Hooks are never copied into the home.
    utils::assert_no_command_hooks_in_home();
}

#[test]
fn sync_hooks_are_not_copied_to_home() {
    conf::init();

    // Regular files.
    conf::create_file_in_configs("foo/pre-sync.sh", None);
    conf::create_file_in_configs("foo/post-sync.sh", None);

    // Hooks.
    conf::create_executable_file_in_configs("pre-sync.sh", None);
    conf::create_executable_file_in_configs("post-sync.sh", None);

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Non-root "hooks" are not hooks, but regular files.
    assert!(files::file_exists_in_home("foo/pre-sync.sh"));
    assert!(files::file_exists_in_home("foo/post-sync.sh"));

    // Hooks are not copied.
    assert!(!files::file_exists_in_home("pre-sync.sh"));
    assert!(!files::file_exists_in_home("post-sync.sh"));
}

#[test]
fn sync_hooks_abort_execution_if_exit_code_is_non_zero() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);

    conf::create_executable_file_in_configs("pre-sync.sh", Some(r"exit 1"));

    let output = run(&["--verbose", "sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    utils::assert_aborted_by(&output, "pre-sync.sh");

    // The aborted `sync` did not touch the home.
    assert!(!files::file_exists_in_home(".gitconfig"));
}

fn create_file_symlink(target: impl AsRef<Path>, symlink: impl AsRef<Path>) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, symlink).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, symlink).unwrap();
}
