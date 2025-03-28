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
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    if let Some(arg) = args.next() {
        return match arg.as_str() {
            "-h" | "--help" => {
                help();
            }
            "-v" | "--version" => {
                version();
            }
            "sync" => {
                sync(args.next());
            }
            "rsync" => {
                rsync(args.next());
            }
            "link" => {
                link(args.next());
            }
            arg => {
                eprintln!("Unknown argument: '{arg}'.\n");
                help();
                process::exit(2);
            }
        };
    }

    // No arguments.

    help();
}

fn help() {
    println!(
        "\
usage: {bin} [<options>] <command> [<args>]

Commands:
  sync [<root>]        ...
  rsync [<root>]       ...
  link [<root>]        ...

Options:
  -h, --help           Show this message and exit.
  -v, --version        Show the version and exit.
",
        bin = env!("CARGO_BIN_NAME"),
    );
}

fn version() {
    println!("{} {}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
}

fn sync(root: Option<String>) {
    todo!("copy files _to_ destination")
}

fn rsync(root: Option<String>) {
    todo!("update files _from_ destination")
}

fn link(root: Option<String>) {
    todo!("symink files _to_ destination")
}
