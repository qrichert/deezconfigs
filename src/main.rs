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
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use deezconfigs::walk;

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
  sync [<root>]    Update system with configs
  rsync [<root>]   Update configs with system
  link [<root>]    Symlink configs to system

Options:
  -h, --help       Show this message and exit.
  -v, --version    Show the version and exit.
",
        bin = env!("CARGO_BIN_NAME"),
    );
}

fn version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

fn sync(root: Option<String>) {
    // TODO: If we want to do Git cloning it's here:
    //  1. Detect git URL.
    //  2. `git clone` to `/tmp` (remove if it exists, no `git pull`, we
    //     don't know the user's config).
    //  3. Normalize `root` to `Some("/tmp/cloned-repo")`.
    //  4. Proceed as usual.

    let root = get_root_or_default(root);
    let home = get_home_directory();

    let nb_files_written = AtomicUsize::new(0);
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, |p| {
        debug_assert!(!p.is_dir());

        let source = root.join(p);
        let destination = home.join(p);

        if let Err(err) = fs::create_dir_all(
            destination
                .parent()
                .expect("at the bare minimum, `parent` is `$HOME`"),
        ) {
            eprintln!("error: Could not copy '{}' to Home: {err}", p.display());
            nb_errors.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // If destination exists and is a symlink, we must _delete_ it
        // before the copy, or else it would override the link's target.
        if destination.is_symlink() {
            if let Err(err) = fs::remove_file(&destination) {
                eprintln!(
                    "error: Could not remove exising symlink '{}': {err}",
                    destination.display()
                );
                nb_errors.fetch_add(1, Ordering::Relaxed);
                return;
            };
        }

        // `fs::copy()` _follows_ symlinks. It will create files with the
        // contents of the symlink's target; it will _not_ create a link.
        if let Err(err) = fs::copy(source, destination) {
            eprintln!("error: Could not copy '{}' to Home: {err}", p.display());
            nb_errors.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // TODO: if verbose { println!("wrote: {}", p.display()) }
        //  (With locked stdout, but looks like it's going to be
        //  a pain... Also check that lock is released _before_
        //  `print_summary()`).
        nb_files_written.fetch_add(1, Ordering::Relaxed);
    });

    print_summary(&root, nb_files_written.into_inner(), nb_errors.into_inner());
}

fn print_summary(root: &Path, nb_files_written: usize, nb_errors: usize) {
    let mut stdout = io::stdout().lock();

    if nb_files_written + nb_errors == 0 {
        _ = writeln!(stdout, "No config files found in '{}'.", root.display());
        return;
    }

    _ = write!(
        stdout,
        "Wrote {nb_files_written} file{}",
        if nb_files_written == 1 { "" } else { "s" }
    );
    if nb_errors > 0 {
        _ = write!(
            stdout,
            ", {nb_errors} error{}",
            if nb_errors == 1 { "" } else { "s" }
        );
    }
    _ = writeln!(stdout, ".");
}

fn rsync(_root: Option<String>) {
    todo!("update files _from_ destination")

    // TODO:
    //  1. Collect all files in `configs`
    //  2. Find matching files in `/home`
    //  3. Replace files in `configs` with files in `/home`.
}

fn link(_root: Option<String>) {
    todo!("symlink files _to_ destination")

    // TODO:
    //  1. For each config file
    //  2. Symlink it to destination
    //     a. ensuring that already existing files are
    //        _replaced_ by symlinks.
}

fn get_root_or_default(root: Option<String>) -> PathBuf {
    if let Some(root) = root {
        let root = PathBuf::from(root);
        if !root.is_dir() {
            eprintln!("fatal: Root must be a valid directory.");
            process::exit(1);
        }
        root
    } else {
        env::current_dir().unwrap_or_else(|_| {
            eprintln!(
                "\
fatal: Could not determine current working directory.
Please provide a Root directory as argument.
"
            );
            process::exit(1);
        })
    }
}

fn get_home_directory() -> PathBuf {
    // TODO: Use `std::env::home_dir()` once it gets un-deprecated.
    let Ok(home_directory) = env::var("HOME") else {
        eprintln!("fatal: Could not read Home directory from environment.");
        process::exit(1);
    };
    PathBuf::from(home_directory)
}
