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
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use deezconfigs::{ui, walk};

use super::common::{
    determine_config_root, get_config_root_from_git, get_home_directory, get_hooks_for_root,
    is_git_remote_uri, run_hooks,
};

/// Sync config from root into Home.
///
/// 1. Collect all files in `configs`.
/// 2. Create or replace matching files in `$HOME`.
pub fn sync(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        determine_config_root(root, true)?
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

        // TODO: Handle case when a directory exists.

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
                    eprintln!(
                        "error: Could not remove exising file '{}': {err}",
                        destination.display()
                    );
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            }

            let target: PathBuf = match fs::read_link(&source) {
                Ok(target) => target,
                Err(err) => {
                    eprintln!("error: Could not read symlink '{}': {err}", p.display());
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            #[cfg(unix)]
            let res = std::os::unix::fs::symlink(&target, &destination);
            #[cfg(windows)]
            let res = std::os::windows::fs::symlink_file(&target, &destination);

            if let Err(err) = res {
                eprintln!("error: Could not create symlink '{}': {err}", p.display());
                nb_errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        } else {
            // If destination exists and is a symlink, we must _delete_
            // it before the copy, or else it would override the link's
            // target.
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

            if let Err(err) = fs::copy(source, destination) {
                eprintln!("error: Could not copy '{}' to Home: {err}", p.display());
                nb_errors.fetch_add(1, Ordering::Relaxed);
                return;
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

    nb_hooks_ran += run_hooks(|| hooks.post_sync(verbose))?;

    ui::print_summary(
        &root,
        nb_files_written.into_inner(),
        nb_errors.into_inner(),
        nb_hooks_ran,
    );

    Ok(())
}
