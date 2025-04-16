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

use super::common::{
    determine_config_root, get_config_root_from_git, get_home_directory, get_hooks_for_root,
    is_git_remote_uri, run_hooks,
};

/// TODO
pub fn clean(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        determine_config_root(root, true)?
    };
    let home = get_home_directory()?;
    let hooks = get_hooks_for_root(&root)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_clean(verbose))?;

    let nb_files_written = AtomicUsize::new(0);
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
                dbg!(&dir);
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
            // Since this is threaded, it's quite a pain to optimise the
            // repeated prints. We can't properly share a stdout lock or
            // a mutable string buffer without jumping through too many
            // hoops. And quite frankly it would be overkill in regard
            // to the limited number of config files we can expect.
            println!("{}", p.display());
        }

        nb_files_written.fetch_add(1, Ordering::Relaxed);
    });

    nb_hooks_ran += run_hooks(|| hooks.post_clean(verbose))?;

    ui::print_summary(
        &root,
        nb_files_written.into_inner(),
        nb_errors.into_inner(),
        nb_hooks_ran,
    );

    Ok(())
}
