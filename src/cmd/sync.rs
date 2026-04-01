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

use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use deezconfigs::{ui, walk};

use super::common::{
    determine_config_root, get_config_root_from_git, get_home_directory, get_hooks_for_command,
    is_git_remote_uri, run_hooks,
};

/// Sync config from root into Home.
///
/// 1. Collect all files in `configs`.
/// 2. Create or replace matching files in `$HOME`.
#[allow(clippy::too_many_lines)] // More a procedure than a function.
pub fn sync(root: Option<&String>, verbose: bool, pull_before_sync: bool) -> Result<(), i32> {
    let root = if pull_before_sync {
        if is_git_remote_uri(root) {
            eprintln!(
                "{fatal}: '--pull' only works with local config roots.",
                fatal = ui::Color::error("fatal")
            );
            return Err(2);
        }
        let root = determine_config_root(root, true)?;
        run_git_pull_in_root(&root)?;
        root
    } else if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        determine_config_root(root, true)?
    };
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_sync())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let files = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_files_synced = AtomicUsize::new(0);
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, |p| {
        debug_assert!(!p.is_dir());

        let source = root.join(p);
        let destination = home.join(p);

        if destination.is_dir() {
            // If destination exists and is a directory, try to `rmdir`
            // it. If it works, the directory was empty anyway. If it
            // doesn't work, the directory is not empty so we abort
            // because it is too risky to remove an entire tree.
            if let Err(err) = fs::remove_dir(&destination) {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not remove exising directory '{}': {err}",
                    destination.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }
        }

        if let Err(err) = fs::create_dir_all(
            destination
                .parent()
                .expect("at the bare minimum, `parent` is `$HOME`"),
        ) {
            nb_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "{error}: Could not copy '{}' to Home: {err}",
                p.display(),
                error = ui::Color::error("error"),
            );
            return;
        }

        // If _source_ is a symlink, copy the link, _not_ the contents.
        // We want to _mirror_ what the user has, not interpret what he
        // might have wanted to do.
        //
        // `fs::copy()` follows symlinks. It will create files with the
        // contents of the symlink's target; it will not create a link.
        if source.is_symlink() {
            // If destination exists we must _delete_ it before the
            // copy, because symlinks don't override existing files.
            if destination.is_file() {
                // Matches both files and symlinks.
                if let Err(err) = fs::remove_file(&destination) {
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "{error}: Could not remove exising file '{}': {err}",
                        destination.display(),
                        error = ui::Color::error("error"),
                    );
                    return;
                }
            }

            let target: PathBuf = match fs::read_link(&source) {
                Ok(target) => target,
                Err(err) => {
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "{error}: Could not read symlink '{}': {err}",
                        p.display(),
                        error = ui::Color::error("error"),
                    );
                    return;
                }
            };

            #[cfg(unix)]
            let res = std::os::unix::fs::symlink(&target, &destination);
            #[cfg(windows)]
            let res = std::os::windows::fs::symlink_file(&target, &destination);

            if let Err(err) = res {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not create symlink '{}': {err}",
                    p.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }
        } else {
            // If destination exists and is a symlink, we must _delete_
            // it before the copy, or else it would override the link's
            // target.
            if destination.is_symlink()
                && let Err(err) = fs::remove_file(&destination)
            {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not remove exising symlink '{}': {err}",
                    destination.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }

            if let Err(err) = fs::copy(source, destination) {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not copy '{}' to Home: {err}",
                    p.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }
        }

        if verbose {
            let file = p.to_string_lossy().to_string();
            if let Ok(mut files) = files.lock() {
                files.push(file);
                // Release the lock ASAP.
                drop(files);
            } else {
                // It's so unlikely we don't acquire the lock that we
                // just silently fall back to printing directly.
                println!("{}", p.display());
            }
        }

        nb_files_synced.fetch_add(1, Ordering::Relaxed);
    });

    let mut files = Arc::try_unwrap(files)
        .expect("processing is over, we're back to a single thread.")
        .into_inner()
        .unwrap();
    // Do not use `sort_unstable()` because the files are likely
    // _partially_ sorted, in which case stable sort is faster,
    // as per the docs.
    files.sort();

    ui::print_files(&files);

    nb_hooks_ran += run_hooks(|| hooks.post_sync())?;

    let nb_files_synced = nb_files_synced.into_inner();
    let nb_errors = nb_errors.into_inner();

    ui::print_summary(
        ui::Action::Sync,
        &root,
        nb_files_synced,
        nb_errors,
        nb_hooks_ran,
    );

    if nb_errors > 0 { Err(1) } else { Ok(()) }
}

/// Run `git pull` inside config root.
///
/// This intentionally does not go through `cmd::run()`. `run` has
/// different semantics; it's meant for users to run arbitrary commands
/// from anywhere, with its own idiosyncratic logic for that.
///
/// Here we only want to run `git pull` inside the root already resolved
/// by `sync`. `sync` has its own logic and evolves at different times
/// and for different reasons than `run` (e.g., `run` needs `DEEZ_ROOT`,
/// we don't. `run` itself asks to confirm the root if no `.deez` file
/// is found, we do too so it would ask twice, etc.).
fn run_git_pull_in_root(root: &Path) -> Result<(), i32> {
    let status = process::Command::new("git")
        .current_dir(root)
        .arg("pull")
        .status();

    match status {
        Ok(status) => match status.code() {
            Some(0) => Ok(()),
            Some(code) => Err(code),
            None => Err(1),
        },
        Err(err) => {
            eprintln!(
                "{fatal}: Could not run 'git pull': {err}",
                fatal = ui::Color::error("fatal")
            );
            Err(1)
        }
    }
}
