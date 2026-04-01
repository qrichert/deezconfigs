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
