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

mod conf;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use conf::HOME;

const DEEZ: &str = env!("CARGO_BIN_EXE_deez");

struct Output {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Output {
    let mut command = Command::new(DEEZ);

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

fn file_exists_in_home(file_path: &str) -> bool {
    let file = PathBuf::from(HOME).join(file_path);
    file.is_file()
}

// fn symlink_exists_in_home(symlink_path: &str) -> bool {
//     let symlink = PathBuf::from(HOME).join(symlink_path);
//     symlink.is_symlink()
// }

#[test]
fn help() {
    let output = run(&["--help"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
    assert!(output.stdout.contains("-V, --version"));
    assert!(output.stdout.contains("-v, --verbose"));
    assert!(output.stdout.contains("sync [<root>|<git>]"));
    assert!(output.stdout.contains("rsync [<root>]"));
    assert!(output.stdout.contains("link [<root>]"));
}

#[test]
fn no_args_shows_help() {
    let output = run(&[]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
}

#[test]
fn version() {
    let output = run(&["--version"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));

    let output = run(&["-V"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn bad_argument() {
    let output = run(&["--bad-argument"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("'--bad-argument'"));
}

#[test]
fn sync_regular() {
    conf::init();

    conf::create_file_in_configs(".gitconfig", None);
    conf::create_file_in_configs(".config/nvim/init.lua", None);
    conf::create_file_in_configs(".config/fish/config.fish", None);
    conf::create_symlink_in_configs(".config/ghostty/config", None);

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    assert!(file_exists_in_home(".gitconfig"));
    assert!(file_exists_in_home(".config/nvim/init.lua"));
    assert!(file_exists_in_home(".config/fish/config.fish"));
    assert!(file_exists_in_home(".config/ghostty/config"));
}

#[test]
fn sync_ignores_special_files() {
    conf::init();

    // OK.
    conf::create_file_in_configs("subdir/.git/config", None);
    conf::create_file_in_configs("subdir/.gitignore", None);
    // NOT OK.
    conf::create_file_in_configs(".gitignore", None);
    conf::create_file_in_configs(".git/config", None);
    // NOT OK, even in subdirectories.
    conf::create_file_in_configs("subdir/.deez", None);

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // OK in sub-directories.
    assert!(file_exists_in_home("subdir/.git/config"));
    assert!(file_exists_in_home("subdir/.gitignore"));
    // NOT OK in root.
    assert!(!file_exists_in_home(".gitignore"));
    assert!(!file_exists_in_home(".git/config"));
    // NOT OK, even in subdirectories.
    assert!(!file_exists_in_home("subdir/.deez"));
}

/// If a file in configs should override a symlink in home, ensure `sync`
/// replaces the symlink with a file, and does _not_ replace the content
/// of the target of the symlink.
#[test]
fn sync_replace_symlink_with_file() {
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
    let content_before = fs::read_to_string(&symlink_in_home).unwrap();
    assert!(symlink_in_home.is_symlink());
    assert_eq!(content_before, "should not be replaced");

    let output = run(&["sync", &conf::root()]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);

    // Ensure the symlink in home is a file now, with updated content.
    let content_after = fs::read_to_string(&symlink_in_home).unwrap();
    assert!(!symlink_in_home.is_symlink()); // `is_file()` traverses.
    assert_eq!(content_after, "new content");

    // Ensure the removed symlink's target has not been altered.
    let symlink_target_content = fs::read_to_string(&symlink_target_in_home).unwrap();
    assert_eq!(symlink_target_content, "should not be replaced");
}
