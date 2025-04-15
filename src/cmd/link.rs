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

/// Link config from root into Home.
///
/// 1. Collect all files in `configs`.
/// 2. Create matching symlinks to the files in `$HOME`.
pub fn link(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = determine_config_root(root, true)?;
    let home = get_home_directory()?;
    let hooks = get_hooks_for_root(&root)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_link(verbose))?;

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
            eprintln!("error: Could not link '{}' to Home: {err}", p.display());
            nb_errors.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // TODO: Handle case when a directory exists.

        // If destination exists, remove it.
        if destination.is_file() || destination.is_symlink() {
            // TODO: We put `is_symlink()` to handle the case when the
            //  link is broken (and so `is_file()` presubably wouldn't
            //  match?). Test it out.
            if let Err(err) = fs::remove_file(&destination) {
                eprintln!(
                    "error: Could not remove exising file '{}': {err}",
                    destination.display()
                );
                nb_errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        }

        #[cfg(unix)]
        let res = std::os::unix::fs::symlink(&source, &destination);
        #[cfg(windows)]
        let res = std::os::windows::fs::symlink_file(&source, &destination);

        if let Err(err) = res {
            eprintln!(
                "error: Could not create link to '{}': {err}",
                source.display()
            );
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

    nb_hooks_ran += run_hooks(|| hooks.post_link(verbose))?;

    ui::print_summary(
        &root,
        nb_files_written.into_inner(),
        nb_errors.into_inner(),
        nb_hooks_ran,
    );

    Ok(())
}
