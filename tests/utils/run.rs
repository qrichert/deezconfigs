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
    run_with_env(args, dir, &[])
}

/// Run `deez` with per-test environment overrides.
///
/// Overrides are applied _after_ the defaults, so `None` can be used to
/// unset something this function sets (e.g., `("NO_COLOR", None)`).
///
/// Variables are set on the child process only. Contrary to
/// [`mock_bin()`](super::mock_bin), this never touches the environment
/// of the test process itself, so tests stay independent of one
/// another.
pub fn run_with_env(args: &[&str], dir: impl AsRef<Path>, envs: &[(&str, Option<&str>)]) -> Output {
    let mut command = Command::new(DEEZ);
    command.current_dir(dir.as_ref());
    command.env("NO_COLOR", "1");
    command.env_remove("PAGER");

    for (key, value) in envs {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }

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
