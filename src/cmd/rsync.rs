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

use deezconfigs::{ui, walk};

use super::common::{determine_config_root, get_home_directory, get_hooks_for_root, run_hooks};

/// Sync config from Home back into root.
///
/// 1. Collect all files in `configs`.
/// 2. Find matching files in `$HOME`.
/// 3. Replace files in `configs` with files in `$HOME`.
pub fn rsync(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = determine_config_root(root)?;
    let home = get_home_directory()?;
    let hooks = get_hooks_for_root(&root)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_rsync(verbose))?;

    let nb_files_written = AtomicUsize::new(0);
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, |p| {
        debug_assert!(!p.is_dir());

        // Despite `rsync` working in reverse, we keep the same
        // terminology as everywhere else for consistency.
        let source = root.join(p);
        let destination = home.join(p);

        // If source exists and is a symlink, we must _delete_ it before
        // the copy, or else it would override the link's target.
        if source.is_symlink() {
            if let Err(err) = fs::remove_file(&source) {
                eprintln!(
                    "error: Could not remove exising symlink '{}': {err}",
                    source.display()
                );
                nb_errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        }

        // `fs::copy()` follows symlinks. It will create files with the
        // contents of the symlink's target; it will not create a link.
        if let Err(err) = fs::copy(destination, source) {
            eprintln!("error: Could not copy '{}' from Home: {err}", p.display());
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

    nb_hooks_ran += run_hooks(|| hooks.post_rsync(verbose))?;

    ui::print_summary(
        &root,
        nb_files_written.into_inner(),
        nb_errors.into_inner(),
        nb_hooks_ran,
    );

    Ok(())
}
