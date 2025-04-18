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

mod cmd;

use std::env;
use std::process;

fn main() {
    let mut verbose = false;

    // TODO: Clean parsing into `Config`.
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        return match arg.as_str() {
            "-h" | "--help" => {
                help();
            }
            "-V" | "--version" => {
                version();
            }
            "sync" | "s" => {
                if let Err(code) = cmd::sync(args.next().as_ref(), verbose) {
                    process::exit(code);
                }
            }
            "rsync" | "rs" => {
                if let Err(code) = cmd::rsync(args.next().as_ref(), verbose) {
                    process::exit(code);
                }
            }
            "link" | "l" => {
                if let Err(code) = cmd::link(args.next().as_ref(), verbose) {
                    process::exit(code);
                }
            }
            "status" | "st" => {
                if let Err(code) = cmd::status(args.next().as_ref(), verbose) {
                    process::exit(code);
                }
            }
            "diff" | "df" => {
                if let Err(code) = cmd::diff(args.next().as_ref(), verbose) {
                    process::exit(code);
                }
            }
            "clean" | "c" => {
                if let Err(code) = cmd::clean(args.next().as_ref(), verbose) {
                    process::exit(code);
                }
            }
            "-v" | "--verbose" => {
                verbose = true;
                continue;
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
Usage: {bin} [<options>] <command> [<args>]

Commands:
  sync [<root>|<git>]    Update Home from configs
  rsync [<root>]         Update configs from Home
  link [<root>]          Symlink configs to Home

  status [<root>|<git>]  List files and their status
  diff [<root>|<git>]    Show what has changed
  clean [<root>|<git>]   Remove all configs from Home

Options:
  -h, --help             Show this message and exit
  -V, --version          Show the version and exit
  -v, --verbose          Show files being copied
",
        bin = env!("CARGO_BIN_NAME"),
    );
}

fn version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}
