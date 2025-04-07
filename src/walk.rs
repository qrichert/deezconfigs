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

use std::ffi::OsStr;
use std::path::Path;

use ignore::{self, DirEntry, WalkBuilder, WalkState};

use crate::hooks::HOOKS;

/// Find files recursively, starting from `root` directory.
///
/// For each file found, call `f(path)`, where `path` is the path of the
/// file, relative to `root`.
///
/// For example, let there be a file `/root/foo`. Calling this function
/// passing `/root` will in turn call `f(&Path::from("foo"))` (the
/// `/root/` prefix got stripped).
///
/// This is useful for us, because it makes it easy to join the same
/// path back to `root`, or, back to `$HOME`, without additional logic.
///
/// # Panics
///
/// This function panics if `root` is not a directory.
pub fn find_files_recursively(root: impl AsRef<Path>, f: impl Fn(&Path) + Sync) {
    let root = root.as_ref();

    let does_dir_entry_match = move |path: &Path| {
        // At the root.
        if [".git"].map(Path::new).contains(&path) {
            return false;
        }

        true
    };

    let does_file_entry_match = move |path: &Path| {
        // At the root.
        let is_at_root = path.components().count() == 1;
        if is_at_root {
            if [".ignore", ".gitignore"].map(Path::new).contains(&path) {
                return false;
            }
            // TODO: Use `PathBuf::file_prefix()` once it lands in stable.
            if let Some(file_name) = path.file_stem() {
                if HOOKS.map(OsStr::new).contains(&file_name) {
                    return false;
                }
            }
        }

        // Anywhere.
        let file_name = path.file_name().expect("we don't have `..` here");
        if [".deez"].map(OsStr::new).contains(&file_name) {
            return false;
        }

        true
    };

    // Note: We want a dir, not a file, but it's not the job of this
    // function to complain to the user.
    assert!(root.is_dir());

    WalkBuilder::new(root)
        .follow_links(false)
        .hidden(false)
        .max_depth(None)
        .build_parallel()
        .run(|| {
            Box::new(|entry| {
                if let Ok(entry) = entry {
                    // Stripping `root` makes `does_entry_match()` much
                    // simpler, and returns a clean path to the caller.
                    let path = strip_root(root, entry.path());

                    if is_dir(&entry) {
                        return if does_dir_entry_match(path) {
                            WalkState::Continue
                        } else {
                            // For example, skip `.git/` dir.
                            WalkState::Skip
                        };
                    }
                    if does_file_entry_match(path) {
                        f(path);
                        return WalkState::Continue;
                    }
                }
                WalkState::Skip
            })
        });
}

#[inline]
fn strip_root<'a>(root: &Path, path: &'a Path) -> &'a Path {
    // Since `root` is the root, `path` _always_ contains `root`.
    // By subtracting `root` from `path`, we get the file path
    // relative to `root`, without including `root`:
    //
    // For `root` == `foo/configs`:
    //
    //   foo/configs/.gitignore -> .gitignore
    //   foo/configs/.config/nvim/init.lua -> .config/nvim/init.lua
    path.strip_prefix(root)
        .expect("`path` always contains `root`")
}

fn is_dir(entry: &DirEntry) -> bool {
    entry.file_type().is_some_and(|entry| entry.is_dir())
}
