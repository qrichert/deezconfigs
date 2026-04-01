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
