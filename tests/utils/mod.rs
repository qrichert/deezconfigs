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

#![allow(dead_code)]

use std::env;
use std::fs;
use std::path::Path;

pub mod conf;
pub mod files;
pub mod run;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
const MOCK_BIN_DIR: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/mock_bin/");
const EMPTY_BIN_DIR: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/empty_bin/");

/// A directory guaranteed to contain no executables.
///
/// Use it as `PATH` (through
/// [`run_with_env()`](run::run_with_env)) to test what happens when an
/// executable cannot be found.
///
/// This is deliberately _not_ [`MOCK_BIN_DIR`]. Mocks are copied there
/// and never removed, so whether a given executable is present depends
/// on which tests ran before.
pub fn empty_bin_dir() -> &'static str {
    let bin_dir = Path::new(EMPTY_BIN_DIR);

    assert!(
        fs::create_dir_all(bin_dir).is_ok(),
        "Error creating empty bin directory: '{}'.",
        bin_dir.display()
    );

    EMPTY_BIN_DIR
}

/// "Monkey-patch" an executable.
///
/// The `fixtures` directory contains shell scripts that mimic the
/// behaviour of executables in different scenarios.
///
/// This function takes the name of one of such mock scripts as input,
/// and plays with the `PATH` environment variable to make this script
/// be executed instead of the real executable.
pub fn mock_bin(bin_name: &str, file: &str) {
    let fixtures_dir = Path::new(FIXTURES_DIR);
    let bin_dir = Path::new(MOCK_BIN_DIR);

    let fixture = fixtures_dir.join(file).with_extension("sh");
    let test_mock = bin_dir.join(bin_name);

    assert!(
        fs::create_dir_all(bin_dir).is_ok(),
        "Error creating mock bin directory: '{}'.",
        bin_dir.display()
    );

    assert!(
        fs::copy(&fixture, test_mock).is_ok(),
        "Error setting up mock executable: '{}'.",
        fixture.display()
    );

    unsafe {
        env::set_var("PATH", format!("{}:/bin:/usr/bin/", bin_dir.display()));
    }
}

/// Read output file created by a mock executable.
///
/// The fixture scripts create output files in the same directory as
/// they're in (i.e., in `target/tmp/mock_bin/`).
pub fn read_output_file(file: &str) -> String {
    let bin_dir = Path::new(MOCK_BIN_DIR);

    fs::read_to_string(bin_dir.join(file).with_extension("txt"))
        .expect("if file doesn't exist, the test failed")
}

pub fn output_file_exists(file: &str) -> bool {
    let bin_dir = Path::new(MOCK_BIN_DIR);

    bin_dir.join(file).with_extension("txt").exists()
}

/// Clean up any stale output file to keep runs isolated.
pub fn remove_output_file(file: &str) {
    let bin_dir = Path::new(MOCK_BIN_DIR);

    let output = bin_dir.join(file).with_extension("txt");
    if output.exists() {
        fs::remove_file(output).unwrap();
    }
}

// Every command with pre- and post-command hooks.
// Keep in sync with `HOOKS` in `src/hooks.rs`.
pub const HOOK_COMMANDS: &[&str] = &["sync", "rsync", "link", "status", "diff", "clean"];

/// Create all command hooks in configs.
pub fn create_all_command_hooks(content: Option<&str>) {
    for c in HOOK_COMMANDS {
        conf::create_executable_file_in_configs(&format!("pre-{c}.sh"), content);
        conf::create_executable_file_in_configs(&format!("post-{c}.sh"), content);
    }
}

/// Create all command hook files in home.
pub fn create_all_command_hooks_in_home(content: Option<&str>) {
    for c in HOOK_COMMANDS {
        conf::create_file_in_home(&format!("pre-{c}.sh"), content);
        conf::create_file_in_home(&format!("post-{c}.sh"), content);
    }
}

/// Assert only hooks for `cmd` ran.
pub fn assert_only_own_hooks_ran(output: &run::Output, cmd: &str) {
    for c in HOOK_COMMANDS {
        let pre = format!("hook: pre-{c}.sh");
        let post = format!("hook: post-{c}.sh");
        if *c == cmd {
            assert!(output.stdout.contains(&pre), "expected '{pre}' to run");
            assert!(output.stdout.contains(&post), "expected '{post}' to run");
        } else {
            assert!(
                !output.stdout.contains(&pre),
                "'{pre}' must not run for '{cmd}'"
            );
            assert!(
                !output.stdout.contains(&post),
                "'{post}' must not run for '{cmd}'"
            );
        }
    }
}

/// Assert command hooks were not written to home.
pub fn assert_no_command_hooks_in_home() {
    for c in HOOK_COMMANDS {
        assert!(!files::file_exists_in_home(&format!("pre-{c}.sh")));
        assert!(!files::file_exists_in_home(&format!("post-{c}.sh")));
    }
}

/// Assert command hooks in home were not removed.
pub fn assert_all_command_hooks_survive_in_home() {
    for c in HOOK_COMMANDS {
        assert!(files::file_exists_in_home(&format!("pre-{c}.sh")));
        assert!(files::file_exists_in_home(&format!("post-{c}.sh")));
    }
}

/// Assert command hooks in configs were not overwritten.
pub fn assert_config_hooks_unchanged(expected: &str) {
    for c in HOOK_COMMANDS {
        assert_eq!(files::read_in_configs(&format!("pre-{c}.sh")), expected);
        assert_eq!(files::read_in_configs(&format!("post-{c}.sh")), expected);
    }
}

/// Assert all command hooks are listed in `output`.
pub fn assert_hooks_section_listed(output: &run::Output) {
    let mut expected = String::from("Hooks\n");
    for c in HOOK_COMMANDS {
        expected.push_str("  pre-");
        expected.push_str(c);
        expected.push_str(".sh\n");
        expected.push_str("  post-");
        expected.push_str(c);
        expected.push_str(".sh\n");
    }
    assert!(
        output.stdout.contains(&expected),
        "expected hooks section:\n{expected}\n--- got stdout: ---\n{}",
        output.stdout
    );
}

/// Assert execution was aborted by `hook`.
pub fn assert_aborted_by(output: &run::Output, hook: &str) {
    assert_eq!(output.exit_code, 1);
    assert!(
        output
            .stderr
            .contains(&format!("abort: Execution aborted by '{hook}'.")),
        "expected abort message for '{hook}', got stderr:\n{}",
        output.stderr
    );
}
