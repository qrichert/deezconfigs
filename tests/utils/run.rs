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
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const DEEZ: &str = env!("CARGO_BIN_EXE_deez");

#[derive(Debug)]
pub struct Output {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(args: &[&str]) -> Output {
    run_in_dir(args, env::current_dir().unwrap())
}

pub fn run_with_input(args: &[&str], input: &str) -> Output {
    run_in_dir_with_input(args, env::current_dir().unwrap(), input)
}

pub fn run_in_dir(args: &[&str], dir: impl AsRef<Path>) -> Output {
    let mut command = Command::new(DEEZ);
    command.current_dir(dir.as_ref());
    command.env("NO_COLOR", "1");
    command.env_remove("PAGER");

    for arg in args {
        command.arg(arg);
    }

    let output = command.output().unwrap();

    Output {
        exit_code: output.status.code().unwrap(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

pub fn run_in_dir_with_input(args: &[&str], dir: impl AsRef<Path>, input: &str) -> Output {
    let mut command = Command::new(DEEZ);
    command.current_dir(dir.as_ref());
    command.env("NO_COLOR", "1");
    command.env_remove("PAGER");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    for arg in args {
        command.arg(arg);
    }

    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();

    Output {
        exit_code: output.status.code().unwrap(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}
