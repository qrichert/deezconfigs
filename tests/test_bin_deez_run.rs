mod utils;

use std::env;

use utils::conf;
use utils::run::run;
use utils::{mock_bin, read_output_file, remove_output_file};

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

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    remove_output_file("output_args");
    remove_output_file("output_pwd");
    mock_bin("mock_cmd", "bin_output_args_and_pwd_to_files");

    let output = run(&["--verbose", "run", "mock_cmd", "foo", "bar"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(output.stdout.contains(&format!("root: {}", conf::root())));
    assert_eq!(read_output_file("output_args").trim(), "foo bar");
    assert_eq!(read_output_file("output_pwd").trim(), conf::root());
}

#[test]
fn run_propagates_command_exit_code() {
    conf::init();

    unsafe {
        env::set_var("DEEZ_ROOT", conf::root());
    }

    mock_bin("mock_cmd", "bin_exit_non_zero");

    let output = run(&["run", "mock_cmd"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 42);
}
