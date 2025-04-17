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

use utils::run::run;

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
