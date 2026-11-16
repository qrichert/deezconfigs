mod utils;

use std::env;
use std::fs;
use std::path::Path;

use utils::conf;
use utils::run::{run_in_dir, run_in_dir_with_input, run_with_env};
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
fn run_runs_command_with_args_in_config_root() {
    conf::init();

    remove_output_file("output_args");
    remove_output_file("output_pwd");
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");

    let output = run_in_dir(
        &["--verbose", "run", "mock_cmd", "foo", "--", "--pull"],
        conf::root(),
    );
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("root: {}", conf::root())));
    assert_eq!(read_output_file("output_args").trim(), "foo -- --pull");
    assert_eq!(read_output_file("output_pwd").trim(), conf::root());
}

#[test]
fn run_propagates_command_exit_code() {
    conf::init();

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    mock_bin("mock_cmd", "bin_exit_non_zero");

    let output = run_in_dir(&["run", "mock_cmd"], conf::HOME);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 42);
}

#[test]
fn run_finds_root_in_parent_directories() {
    conf::init();
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");

    for (command, directory) in [("run", "one"), ("r", "one/two")] {
        let cwd = conf::create_dir_in_configs(directory);
        remove_output_file("output_pwd");

        let output = run_in_dir(&[command, "mock_cmd"], cwd);

        assert_eq!(output.exit_code, 0);
        assert_eq!(read_output_file("output_pwd").trim(), conf::root());
        assert!(!output.stdout.contains("Proceed?"));
    }
}

#[test]
fn run_prefers_nearest_marked_root_over_environment() {
    conf::init();
    let inner = conf::create_dir_in_configs("inner");
    conf::create_file_in_configs("inner/.deez", None);
    let cwd = conf::create_dir_in_configs("inner/deep");
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");
    remove_output_file("output_pwd");

    let output = run_with_env(
        &["run", "mock_cmd"],
        cwd,
        &[("DEEZ_ROOT", Some("/this/deez/root/does/not/exist"))],
    );

    assert_eq!(output.exit_code, 0);
    assert_eq!(
        read_output_file("output_pwd").trim(),
        inner.display().to_string()
    );
}

#[test]
fn run_uses_marked_environment_root_outside_a_root() {
    conf::init();
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");
    remove_output_file("output_pwd");
    let root = conf::root();

    let output = run_with_env(
        &["run", "mock_cmd"],
        conf::HOME,
        &[("DEEZ_ROOT", Some(&root))],
    );

    assert_eq!(output.exit_code, 0);
    assert_eq!(read_output_file("output_pwd").trim(), root);
    assert!(!output.stdout.contains("Proceed?"));
}

#[test]
fn run_confirms_unmarked_current_directory() {
    conf::init();
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");
    remove_output_file("output_pwd");

    let output = run_in_dir_with_input(&["run", "mock_cmd"], conf::HOME, "y\n");

    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout.matches("Proceed? (y/N) ").count(), 1);
    assert!(
        output
            .stderr
            .contains("warning: `root` is not a configuration root.")
    );
    assert_eq!(read_output_file("output_pwd").trim(), conf::HOME);
}

#[test]
fn run_confirms_unmarked_environment_root() {
    conf::init();
    let unmarked = conf::create_dir_in_configs("unmarked");
    unsafe { env::set_var("DEEZ_ROOT", &unmarked) };
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");
    remove_output_file("output_pwd");

    let output = run_in_dir_with_input(&["run", "mock_cmd"], conf::HOME, "yes\n");

    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout.matches("Proceed? (y/N) ").count(), 1);
    assert_eq!(
        read_output_file("output_pwd").trim(),
        unmarked.display().to_string()
    );
}

#[test]
fn run_does_not_execute_when_unmarked_root_is_declined_or_input_ends() {
    conf::init();
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");
    remove_output_file("output_pwd");
    let refused = run_in_dir_with_input(&["run", "mock_cmd"], conf::HOME, "n\n");
    assert_eq!(refused.exit_code, 2);
    assert_eq!(refused.stdout.matches("Proceed? (y/N) ").count(), 1);
    assert!(!output_file_exists("output_pwd"));

    let unmarked = conf::create_dir_in_configs("unmarked");
    unsafe { env::set_var("DEEZ_ROOT", unmarked) };
    let eof = run_in_dir_with_input(&["run", "mock_cmd"], conf::HOME, "");
    assert_eq!(eof.exit_code, 2);
    assert_eq!(eof.stdout.matches("Proceed? (y/N) ").count(), 1);
    assert!(!output_file_exists("output_pwd"));
}

#[test]
fn run_rejects_invalid_environment_roots() {
    conf::init();
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");
    let file = Path::new(conf::HOME).join("file");
    fs::write(&file, "not a directory").unwrap();

    for root in ["/this/deez/root/does/not/exist", file.to_str().unwrap()] {
        remove_output_file("output_pwd");
        let output = run_with_env(
            &["run", "mock_cmd"],
            conf::HOME,
            &[("DEEZ_ROOT", Some(root))],
        );

        assert_eq!(output.exit_code, 1);
        assert!(output.stderr.contains("Root must be a valid directory."));
        assert!(!output_file_exists("output_pwd"));
    }
}

#[test]
fn bare_run_reports_missing_command_before_resolving_root() {
    conf::init();

    for root in [None, Some("/this/deez/root/does/not/exist")] {
        let output = run_with_env(&["run"], conf::HOME, &[("DEEZ_ROOT", root)]);

        assert_eq!(output.exit_code, 2);
        assert!(output.stderr.contains("Run deez what?"));
        assert!(!output.stdout.contains("Proceed?"));
    }
}
