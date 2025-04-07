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
use std::path::{Path, PathBuf};

use deezconfigs::hooks::Hooks;
use deezconfigs::ui;

/// Determine config root for a given path.
///
/// The root is either provided by the user, or we use a heuristic to
/// find an appropriate one to use:
///
/// 1. Get current working directory (cwd).
/// 2. If cwd is not a config root, look into its parents.
/// 3. If none of its parents is a config route, keep using cwd.
///
/// In any case, the selected root is then checked to ensure it is a
/// config root (contains a `.deez` file). If not, we let the user
/// choose to use it anyway, or to abort.
///
/// This check is essential to make, because otherwise the user may
/// inadvertently mess up his Home directory by syncing the wrong root.
pub fn determine_config_root(root: Option<&String>) -> Result<PathBuf, i32> {
    let root = if let Some(root) = root {
        get_config_root_from_args(root)?
    } else {
        let mut default = get_default_config_root()?;
        if !is_a_config_root(&default) {
            if let Some(parent) = find_config_root_in_parents(&default) {
                default = parent.to_path_buf();
            }
        }
        default
    };
    ensure_root_is_a_config_root(&root)?;
    Ok(root)
}

fn get_config_root_from_args(root: &str) -> Result<PathBuf, i32> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        eprintln!("fatal: Root must be a valid directory.");
        if root.is_file() {
            eprintln!("'{}' is a file.", root.display());
        } else if !root.exists() {
            if root.to_str().is_some_and(str::is_empty) {
                eprintln!("No path provided.");
            } else {
                eprintln!("'{}' does not exist.", root.display());
            }
        }
        return Err(1);
    }
    Ok(root)
}

fn get_default_config_root() -> Result<PathBuf, i32> {
    let Ok(root) = env::current_dir() else {
        eprint!(
            "\
fatal: Could not determine current working directory.
Please provide a Root directory as argument.
"
        );
        return Err(1);
    };
    Ok(root)
}

fn find_config_root_in_parents(root: &Path) -> Option<&Path> {
    const DEPTH_LIMIT: usize = 20;
    // `skip()` self.
    for (i, candidate) in root.ancestors().skip(1).enumerate() {
        if is_a_config_root(candidate) {
            return Some(candidate);
        }
        if i == DEPTH_LIMIT {
            break;
        }
    }
    None
}

/// Ensure `root` holds config and is not a random directory.
///
/// To be a config root, the directory must contain a `.deez` file, or
/// the user must give confirmation.
fn ensure_root_is_a_config_root(root: &Path) -> Result<(), i32> {
    if is_a_config_root(root) {
        return Ok(());
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

    if ui::ask_confirmation_with_prompt("Proceed?") {
        println!();
        return Ok(());
    }

    eprintln!("Aborting.");

    Err(2)
}

fn is_a_config_root(root: &Path) -> bool {
    root.join(".deez").is_file()
}

/// Get the user's Home directory.
///
/// The Home directory is read from `HOME` environment variable.
pub fn get_home_directory() -> Result<PathBuf, i32> {
    // TODO: Use `std::env::home_dir()` once it gets un-deprecated.
    if let Ok(home_directory) = env::var("HOME") {
        Ok(PathBuf::from(home_directory))
    } else {
        eprintln!("fatal: Could not read Home directory from environment.");
        Err(1)
    }
}

/// Helper function to instantiate [`Hooks`] from a root, or error.
pub fn get_hooks_for_root(root: &Path) -> Result<Hooks, i32> {
    match Hooks::for_root(root) {
        Ok(hooks) => Ok(hooks),
        Err(err) => {
            eprintln!("fatal: {err}");
            Err(1)
        }
    }
}

/// Helper function to run a group of hooks, or error.
pub fn run_hooks(hooks: impl Fn() -> Result<usize, &'static str>) -> Result<usize, i32> {
    match hooks() {
        Ok(nb_hooks) => Ok(nb_hooks),
        Err(err) => {
            eprintln!("fatal: {err}");
            Err(1)
        }
    }
}
