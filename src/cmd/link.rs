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

use super::common::{determine_config_root, get_home_directory, get_hooks_for_command, run_hooks};

/// Link config from root into Home.
///
/// 1. Collect all files in `configs`.
/// 2. Create matching symlinks to the files in `$HOME`.
pub fn link(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = determine_config_root(root, true)?;
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_link())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let files = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_files_linked = AtomicUsize::new(0);
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
            nb_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("error: Could not link '{}' to Home: {err}", p.display());
            return;
        }

        // TODO: Handle case when a directory exists.

        // If destination exists, remove it.
        if destination.is_file() || destination.is_symlink() {
            // TODO: We put `is_symlink()` to handle the case when the
            //  link is broken (and so `is_file()` presubably wouldn't
            //  match?). Test it out.
            if let Err(err) = fs::remove_file(&destination) {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "error: Could not remove exising file '{}': {err}",
                    destination.display()
                );
                return;
            }
        }

        #[cfg(unix)]
        let res = std::os::unix::fs::symlink(&source, &destination);
        #[cfg(windows)]
        let res = std::os::windows::fs::symlink_file(&source, &destination);

        if let Err(err) = res {
            nb_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "error: Could not create link to '{}': {err}",
                source.display()
            );
            return;
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

        nb_files_linked.fetch_add(1, Ordering::Relaxed);
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

    nb_hooks_ran += run_hooks(|| hooks.post_link())?;

    ui::print_summary(
        ui::Action::Link,
        &root,
        nb_files_linked.into_inner(),
        nb_errors.into_inner(),
        nb_hooks_ran,
    );

    Ok(())
}
