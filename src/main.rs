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

use deezconfigs::{utils, walk};

fn main() {
    let mut verbose = false;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                help();
            }
            "-V" | "--version" => {
                version();
            }
            "sync" => {
                sync(args.next(), verbose);
            }
            "rsync" => {
                rsync(args.next(), verbose);
            }
            "link" => {
                link(args.next(), verbose);
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
        // `return`, unless `continue`.
        return;
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
  -V, --version    Show the version and exit.
  -v, --verbose    Show files being copied.
",
        bin = env!("CARGO_BIN_NAME"),
    );
}

fn version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

fn sync(root: Option<String>, verbose: bool) {
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

        if verbose {
            // Since this is threaded, it's quite a pain to optimise the
            // repeated prints. We can't properly share a stdout lock or
            // a mutable string buffer without jumping through too many
            // hoops. And quite frankly it would be overkill in regard
            // to the limited number of config files we can expect.
            println!("{}", p.display());
        }

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

fn rsync(_root: Option<String>, _verbose: bool) {
    todo!("update files _from_ destination")

    // TODO:
    //  1. Collect all files in `configs`
    //  2. Find matching files in `/home`
    //  3. Replace files in `configs` with files in `/home`.
}

fn link(_root: Option<String>, _verbose: bool) {
    todo!("symlink files _to_ destination")

    // TODO:
    //  1. For each config file
    //  2. Symlink it to destination
    //     a. ensuring that already existing files are
    //        _replaced_ by symlinks.
}

fn get_root_or_default(root: Option<String>) -> PathBuf {
    let root = if let Some(root) = root {
        let root = PathBuf::from(root);
        if !root.is_dir() {
            eprintln!("fatal: Root must be a valid directory.");
            if root.is_file() {
                eprintln!("'{}' is a file.", root.display());
            } else if !root.exists() {
                eprintln!("'{}' does not exist.", root.display());
            }
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
    };

    ensure_root_is_a_config_root(&root);

    root
}

fn get_home_directory() -> PathBuf {
    // TODO: Use `std::env::home_dir()` once it gets un-deprecated.
    let Ok(home_directory) = env::var("HOME") else {
        eprintln!("fatal: Could not read Home directory from environment.");
        process::exit(1);
    };
    PathBuf::from(home_directory)
}

/// Ensure `root` holds config and is not a random directory.
///
/// To be a config root, the directory must contain a `.deez` file, or
/// the user must give confirmation.
fn ensure_root_is_a_config_root(root: &Path) {
    if root.join(".deez").is_file() {
        return;
    }

    eprint!(
        "\
warning: `root` is not a configuration root.

To make it a configuration root, create a `.deez` file inside of it.
This is a security feature. `{bin}` doesn't want to mess up your Home
directory if you run it at the wrong root.

Selected root: '{}'.

",
        root.display(),
        bin = env!("CARGO_BIN_NAME"),
    );

    if utils::ask_confirmation_with_prompt("Proceed?") {
        println!();
        return;
    }

    eprintln!("Aborting.");
    process::exit(2);
}
