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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use deezconfigs::{ui, walk};

use super::common::{
    determine_config_root, get_config_root_from_git, get_home_directory, get_hooks_for_command,
    is_git_remote_uri, run_hooks,
};

/// Remove config files from Home.
///
/// 1. Collect all files in `configs`.
/// 2. Remove matching files in `$HOME`.
pub fn clean(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        determine_config_root(root, true)?
    };
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_clean())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let files = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_files_removed = AtomicUsize::new(0);
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, |p| {
        debug_assert!(!p.is_dir());

        let destination = home.join(p);

        // TODO: Handle case when a directory exists.

        // Matches both files and symlinks.
        if destination.is_file() {
            if let Err(err) = fs::remove_file(&destination) {
                eprintln!(
                    "error: Could not remove file '{}': {err}",
                    destination.display()
                );
                nb_errors.fetch_add(1, Ordering::Relaxed);
                return;
            }

            // Remove all parent dirs until not empty or Home.
            #[allow(clippy::items_after_statements)]
            const DEPTH_LIMIT: usize = 20;
            // `skip()` self (file).
            for (i, dir) in destination.ancestors().skip(1).enumerate() {
                // Don't remove Home or above.
                if dir == home {
                    break;
                }
                // Basically, remove until it fails, since it fails if
                // `dir` is not empty.
                if fs::remove_dir(dir).is_err() {
                    break;
                }
                if i == DEPTH_LIMIT {
                    break;
                }
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

        nb_files_removed.fetch_add(1, Ordering::Relaxed);
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

    nb_hooks_ran += run_hooks(|| hooks.post_clean())?;

    ui::print_summary(
        ui::Action::Clean,
        &root,
        nb_files_removed.into_inner(),
        nb_errors.into_inner(),
        nb_hooks_ran,
    );

    Ok(())
}
