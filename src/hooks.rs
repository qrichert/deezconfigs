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

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process;

const HOOKS: [&str; 12] = [
    "pre-sync",
    "post-sync",
    "pre-rsync",
    "post-rsync",
    "pre-link",
    "post-link",
    "pre-status",
    "post-status",
    "pre-diff",
    "post-diff",
    "pre-clean",
    "post-clean",
];

pub fn is_hook(path: &Path) -> bool {
    if let Some(file_prefix) = crate::utils::file_prefix(path) {
        return HOOKS.map(OsStr::new).contains(&file_prefix);
    }
    false
}

#[derive(Debug)]
struct Scripts {
    pre_sync: Vec<PathBuf>,
    post_sync: Vec<PathBuf>,
    pre_rsync: Vec<PathBuf>,
    post_rsync: Vec<PathBuf>,
    pre_link: Vec<PathBuf>,
    post_link: Vec<PathBuf>,
    pre_status: Vec<PathBuf>,
    post_status: Vec<PathBuf>,
    pre_diff: Vec<PathBuf>,
    post_diff: Vec<PathBuf>,
    pre_clean: Vec<PathBuf>,
    post_clean: Vec<PathBuf>,
}

/// Context for hooks for a given root.
#[derive(Debug)]
pub struct Hooks<'a> {
    root: &'a Path,
    home: &'a Path,
    is_verbose: bool,
    envs: HashMap<&'static str, OsString>,
    scripts: Scripts,
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
    pub fn for_command(
        root: &'a Path,
        home: &'a Path,
        verbose: bool,
    ) -> Result<Self, &'static str> {
        let mut hooks = Self {
            root,
            home,
            is_verbose: verbose,
            envs: HashMap::new(),
            scripts: Scripts {
                pre_sync: Vec::new(),
                post_sync: Vec::new(),
                pre_rsync: Vec::new(),
                post_rsync: Vec::new(),
                pre_link: Vec::new(),
                post_link: Vec::new(),
                pre_status: Vec::new(),
                post_status: Vec::new(),
                pre_diff: Vec::new(),
                post_diff: Vec::new(),
                pre_clean: Vec::new(),
                post_clean: Vec::new(),
            },
        };

        Self::populate_hooks_scripts(&mut hooks)?;
        Self::sort_hooks_scripts_by_file_name(&mut hooks);

        Self::build_environment(&mut hooks);

        Ok(hooks)
    }

    fn populate_hooks_scripts(hooks: &mut Hooks) -> Result<(), &'static str> {
        let Ok(entries) = hooks.root.read_dir() else {
            return Err("Could not read root directory for hooks.");
        };

        for entry in entries.filter_map(|entry| entry.map(|e| e.path()).ok()) {
            if !entry.is_file() {
                continue;
            }

            // This is for normalization and consistency with `walk`
            // returning paths relative to `root`.
            let Ok(entry) = entry.strip_prefix(hooks.root) else {
                // `expect()` whines about missing panics doc.
                unreachable!("we are inside `root`");
            };

            let Some(file_prefix) = crate::utils::file_prefix(entry) else {
                continue;
            };

            match file_prefix.to_str() {
                Some("pre-sync") => hooks.scripts.pre_sync.push(entry.to_path_buf()),
                Some("post-sync") => hooks.scripts.post_sync.push(entry.to_path_buf()),
                Some("pre-rsync") => hooks.scripts.pre_rsync.push(entry.to_path_buf()),
                Some("post-rsync") => hooks.scripts.post_rsync.push(entry.to_path_buf()),
                Some("pre-link") => hooks.scripts.pre_link.push(entry.to_path_buf()),
                Some("post-link") => hooks.scripts.post_link.push(entry.to_path_buf()),
                Some("pre-status") => hooks.scripts.pre_status.push(entry.to_path_buf()),
                Some("post-status") => hooks.scripts.post_status.push(entry.to_path_buf()),
                Some("pre-diff") => hooks.scripts.pre_diff.push(entry.to_path_buf()),
                Some("post-diff") => hooks.scripts.post_diff.push(entry.to_path_buf()),
                Some("pre-clean") => hooks.scripts.pre_clean.push(entry.to_path_buf()),
                Some("post-clean") => hooks.scripts.post_clean.push(entry.to_path_buf()),
                _ => {}
            }
        }

        Ok(())
    }

    fn sort_hooks_scripts_by_file_name(hooks: &mut Hooks) {
        // Do not use `sort_unstable()` because the files are likely
        // _partially_ sorted, in which case stable sort is faster,
        // as per the docs.

        // TODO: Test this behaviour.
        hooks.scripts.pre_sync.sort();
        hooks.scripts.post_sync.sort();
        hooks.scripts.pre_rsync.sort();
        hooks.scripts.post_rsync.sort();
        hooks.scripts.pre_link.sort();
        hooks.scripts.post_link.sort();
        hooks.scripts.pre_status.sort();
        hooks.scripts.post_status.sort();
        hooks.scripts.pre_diff.sort();
        hooks.scripts.post_diff.sort();
        hooks.scripts.pre_clean.sort();
        hooks.scripts.post_clean.sort();
    }

    fn build_environment(hooks: &mut Hooks) {
        // TODO: Add target system (linux, mac, windows, other).
        hooks.set_env_var(
            "DEEZ_ROOT",
            hooks
                .root
                .canonicalize()
                .unwrap_or_else(|_| hooks.root.to_path_buf())
                .as_os_str(),
        );
        hooks.set_env_var(
            "DEEZ_HOME",
            hooks
                .home
                .canonicalize()
                .unwrap_or_else(|_| hooks.home.to_path_buf())
                .as_os_str(),
        );
        if hooks.is_verbose {
            hooks.set_env_var("DEEZ_VERBOSE", OsStr::new("true"));
        }
    }

    pub fn set_env_var(&mut self, key: &'static str, value: impl AsRef<OsStr>) {
        self.envs.insert(key, value.as_ref().to_os_string());
    }

    /// Run "pre-sync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_sync(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.pre_sync)
    }

    /// Run "post-sync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_sync(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.post_sync)
    }

    /// Run "pre-rsync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_rsync(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.pre_rsync)
    }

    /// Run "post-rsync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_rsync(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.post_rsync)
    }

    /// Run "pre-link" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_link(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.pre_link)
    }

    /// Run "post-link" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_link(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.post_link)
    }

    /// Run "pre-status" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_status(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.pre_status)
    }

    /// Run "post-status" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_status(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.post_status)
    }

    /// Run "pre-diff" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_diff(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.pre_diff)
    }

    /// Run "post-diff" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_diff(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.post_diff)
    }

    /// Run "pre-clean" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_clean(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.pre_clean)
    }

    /// Run "post-clean" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_clean(&self) -> Result<usize, &'static str> {
        self.run_hooks(&self.scripts.post_clean)
    }

    fn run_hooks(&self, hooks: &[PathBuf]) -> Result<usize, &'static str> {
        for hook in hooks {
            self.run_hook(hook)?;
        }
        Ok(hooks.len())
    }

    fn run_hook(&self, hook: &Path) -> Result<(), &'static str> {
        // `root` cannot be an empty `Path`.
        debug_assert!(!self.root.to_str().is_some_and(str::is_empty));

        if self.is_verbose {
            println!("hook: {}", hook.display());
        }

        let status = process::Command::new("sh")
            .arg("-c")
            .arg(self.root.join(hook)) // Always a path (`root` non-empty).
            .envs(&self.envs)
            .current_dir(self.root)
            .status();
        if status.is_err() {
            return Err("Could not find the 'sh' executable.");
        }

        Ok(())
    }

    /// List of hooks, grouped by type, and in execution order.
    ///
    /// Hooks are already sorted in lexicographic order, that also
    /// determines the order of execution.
    #[must_use]
    pub fn list(&self) -> Vec<Cow<str>> {
        self.scripts
            .pre_sync
            .iter()
            .chain(self.scripts.post_sync.iter())
            .chain(self.scripts.pre_rsync.iter())
            .chain(self.scripts.post_rsync.iter())
            .chain(self.scripts.pre_link.iter())
            .chain(self.scripts.post_link.iter())
            .chain(self.scripts.pre_status.iter())
            .chain(self.scripts.post_status.iter())
            .chain(self.scripts.pre_diff.iter())
            .chain(self.scripts.post_diff.iter())
            .chain(self.scripts.pre_clean.iter())
            .chain(self.scripts.post_clean.iter())
            .map(|hook| hook.to_string_lossy())
            .collect()
    }
}
