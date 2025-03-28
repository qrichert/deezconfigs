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

use std::env;
use std::path::Path;
use std::process::Command;

const DEEZ: &str = env!("CARGO_BIN_EXE_deez");

struct Output {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn run(dir: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(DEEZ);

    for arg in args {
        command.arg(arg);
    }

    let output = command.current_dir(dir).output().unwrap();

    Output {
        exit_code: output.status.code().unwrap(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

#[test]
fn help() {
    let output = run(&env::temp_dir(), &["--help"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
    assert!(output.stdout.contains("-v, --version"));
}

#[test]
fn no_args_shows_help() {
    let output = run(&env::temp_dir(), &[]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
}

#[test]
fn version() {
    let output = run(&env::temp_dir(), &["--version"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn bad_argument() {
    let output = run(&env::temp_dir(), &["--bad-argument"]);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("'--bad-argument'"));
}
