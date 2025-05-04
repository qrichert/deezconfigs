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
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use deezconfigs::{ui, walk};

use super::common::{determine_config_root, get_home_directory, get_hooks_for_command, run_hooks};

/// Sync config from Home back into root.
///
/// 1. Collect all files in `configs`.
/// 2. Find matching files in `$HOME`.
/// 3. Replace files in `configs` with files in `$HOME`.
pub fn rsync(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = determine_config_root(root, true)?;
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_rsync())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let files = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_files_rsynced = AtomicUsize::new(0);
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, |p| {
        debug_assert!(!p.is_dir());

        // Despite `rsync` working in reverse, we keep the same
        // terminology as everywhere else for consistency.
        let source = root.join(p);
        let destination = home.join(p);

        // Note: Here won't don't worry about `source` being a directory
        // because it can't be. If it was, `find_files_recursively()`
        // would not yield it.

        if destination.is_symlink()
            && match does_symlink_point_to_file(&destination, &source) {
                Ok(points_to_source) => points_to_source,
                Err(err) => {
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("{err}");
                    return;
                }
            }
        {
            // No-op.
            //
            // If a symlink in Home links to a file in configs, copying
            // it back to configs (i.e, `cp B A` where `B@ -> A`) would
            // (likely) truncate the file. This behaviour is documented
            // in `std::fs::copy()` (Rust 1.86) and observed at least on
            // macOS. This is a no-op for us since a symlink is always
            // up-to-date.
        } else if destination.is_file() {
            // Follows symlinks.
            // `fs::copy()` follows symlinks. It will create files with
            // the contents of the symlink's target; it will not create
            // a link.
            if let Err(err) = fs::copy(destination, source) {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not copy '{}' from Home: {err}",
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

        nb_files_rsynced.fetch_add(1, Ordering::Relaxed);
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

    nb_hooks_ran += run_hooks(|| hooks.post_rsync())?;

    let nb_files_rsynced = nb_files_rsynced.into_inner();
    let nb_errors = nb_errors.into_inner();

    ui::print_summary(
        ui::Action::RSync,
        &root,
        nb_files_rsynced,
        nb_errors,
        nb_hooks_ran,
    );

    if nb_errors > 0 { Err(1) } else { Ok(()) }
}

fn does_symlink_point_to_file(symlink: &Path, file: &Path) -> Result<bool, String> {
    let symlink_target = match fs::read_link(symlink) {
        Ok(target) => target,
        Err(err) => {
            return Err(format!(
                "{error}: Symbolic link is broken '{}': {err}",
                symlink.display(),
                error = ui::Color::error("error"),
            ));
        }
    };

    let symlink_target = match symlink_target.canonicalize() {
        Ok(target) => target,
        Err(err) => {
            return Err(format!(
                "{error}: Could not resolve '{}': {err}",
                symlink_target.display(),
                error = ui::Color::error("error"),
            ));
        }
    };
    let file = match file.canonicalize() {
        Ok(file) => file,
        Err(err) => {
            return Err(format!(
                "{error}: Could not resolve '{}': {err}",
                file.display(),
                error = ui::Color::error("error"),
            ));
        }
    };

    Ok(symlink_target == file)
}
