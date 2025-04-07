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

use std::path::{Path, PathBuf};
use std::process;

pub(crate) const HOOKS: [&str; 6] = [
    "pre-sync",
    "post-sync",
    "pre-rsync",
    "post-rsync",
    "pre-link",
    "post-link",
];

#[derive(Debug)]
pub struct Hooks<'a> {
    root: &'a Path,
    pre_sync: Vec<PathBuf>,
    post_sync: Vec<PathBuf>,
    pre_rsync: Vec<PathBuf>,
    post_rsync: Vec<PathBuf>,
    pre_link: Vec<PathBuf>,
    post_link: Vec<PathBuf>,
}

impl<'a> Hooks<'a> {
    /// Create hooks handler given config root.
    ///
    /// This function will look for hooks in the config root and
    /// register them in the handler.
    ///
    /// # Errors
    ///
    /// Errors if the config root cannot be read.
    pub fn for_root(root: &'a Path) -> Result<Self, &'static str> {
        let mut hooks = Self {
            root,
            pre_sync: Vec::new(),
            post_sync: Vec::new(),
            pre_rsync: Vec::new(),
            post_rsync: Vec::new(),
            pre_link: Vec::new(),
            post_link: Vec::new(),
        };

        let Ok(entries) = root.read_dir() else {
            return Err("Could not read root directory for hooks.");
        };

        for entry in entries.filter_map(|entry| entry.map(|e| e.path()).ok()) {
            if !entry.is_file() {
                continue;
            }

            // This is for normalization and consistency with `walk`
            // returning paths relative to `root`.
            let Ok(entry) = entry.strip_prefix(root) else {
                // `expect()` whines about missing panics doc.
                unreachable!("we are inside `root`");
            };

            // TODO: Use `PathBuf::file_prefix()` once it lands in stable.
            let Some(file_name) = entry.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };

            match file_name {
                "pre-sync" => hooks.pre_sync.push(entry.to_path_buf()),
                "post-sync" => hooks.post_sync.push(entry.to_path_buf()),
                "pre-rsync" => hooks.pre_rsync.push(entry.to_path_buf()),
                "post-rsync" => hooks.post_rsync.push(entry.to_path_buf()),
                "pre-link" => hooks.pre_link.push(entry.to_path_buf()),
                "post-link" => hooks.post_link.push(entry.to_path_buf()),
                _ => {}
            }
        }

        Ok(hooks)
    }

    /// Run "pre-sync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_sync(&self, verbose: bool) -> Result<usize, &'static str> {
        self.run_hooks(&self.pre_sync, verbose)
    }

    /// Run "post-sync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_sync(&self, verbose: bool) -> Result<usize, &'static str> {
        self.run_hooks(&self.post_sync, verbose)
    }

    /// Run "pre-rsync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_rsync(&self, verbose: bool) -> Result<usize, &'static str> {
        self.run_hooks(&self.pre_rsync, verbose)
    }

    /// Run "post-rsync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_rsync(&self, verbose: bool) -> Result<usize, &'static str> {
        self.run_hooks(&self.post_rsync, verbose)
    }

    /// Run "pre-link" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_link(&self, verbose: bool) -> Result<usize, &'static str> {
        self.run_hooks(&self.pre_link, verbose)
    }

    /// Run "post-link" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_link(&self, verbose: bool) -> Result<usize, &'static str> {
        self.run_hooks(&self.post_link, verbose)
    }

    fn run_hooks(&self, hooks: &[PathBuf], verbose: bool) -> Result<usize, &'static str> {
        for hook in hooks {
            self.run_hook(hook, verbose)?;
        }
        Ok(hooks.len())
    }

    fn run_hook(&self, hook: &Path, verbose: bool) -> Result<(), &'static str> {
        // `root` cannot be an empty `Path`.
        debug_assert!(!self.root.to_str().is_some_and(str::is_empty));

        if verbose {
            println!("hook: {}", hook.display());
        }
        let status = process::Command::new("sh")
            .arg("-c")
            .arg(self.root.join(hook)) // Always a path (`root` non-empty).
            .current_dir(self.root)
            .status();
        if status.is_err() {
            return Err("Could not find the 'sh' executable.");
        }
        Ok(())
    }
}
