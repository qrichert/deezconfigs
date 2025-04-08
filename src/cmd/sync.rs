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
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use deezconfigs::{ui, walk};

use super::common::{determine_config_root, get_home_directory, get_hooks_for_root, run_hooks};

pub fn sync(root: Option<String>, verbose: bool) -> Result<(), i32> {
    let root = if is_git_remote_uri(root.as_ref()) {
        get_config_root_from_git(&root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        determine_config_root(root.as_ref())?
    };
    let home = get_home_directory()?;
    let hooks = get_hooks_for_root(&root)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_sync(verbose))?;

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
            }
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

    nb_hooks_ran += run_hooks(|| hooks.post_sync(verbose))?;

    ui::print_summary(
        &root,
        nb_files_written.into_inner(),
        nb_errors.into_inner(),
        nb_hooks_ran,
    );

    Ok(())
}

fn is_git_remote_uri(root: Option<&String>) -> bool {
    root.as_ref().is_some_and(|r| r.starts_with("git:"))
}

fn get_config_root_from_git(uri: &str, verbose: bool) -> Result<PathBuf, i32> {
    let uri = uri.trim_start_matches("git:").to_string();

    // Yes, I know. Not a solid UUID, I should use a crate, etc.
    let uuid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time > Unix epoch")
        .as_millis()
        .to_string();
    let clone_path = env::temp_dir().join(format!("deez-{uuid}"));

    if clone_path.is_dir() && fs::remove_dir_all(&clone_path).is_err() {
        eprint!(
            "\
fatal: Could not clone the configuration repository.
The target directory already exists and could not be deleted.
"
        );
        return Err(1);
    }

    println!("Fetching config files remotely...");

    let mut command = process::Command::new("git");
    command
        .env("LANG", "en_US.UTF-8")
        .arg("clone")
        .arg("--single-branch")
        .arg("--depth=1")
        .arg("--no-tags")
        .arg(&uri)
        .arg(&clone_path);

    let status = if verbose {
        command.status().ok()
    } else {
        command.arg("--quiet");
        command.output().ok().map(|out| out.status)
    };

    if let Some(status) = status {
        if !status.success() {
            eprintln!("fatal: Could not clone the configuration repository.");
            if !verbose {
                eprintln!("Retry with `--verbose` for additional detail.");
            }
            return Err(1);
        }
    } else {
        eprint!(
            "\
fatal: Could not clone the configuration repository.
Did not find the 'git' executable. Please ensure Git is properly
installed on your machine.
"
        );
        return Err(1);
    }

    println!("Done.");

    Ok(clone_path)
}
