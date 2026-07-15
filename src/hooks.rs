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
use std::{env, process};

use crate::ui;

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
    if let Some(file_prefix) = path.file_prefix() {
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
    pub fn for_command(root: &'a Path, home: &'a Path, verbose: bool) -> Result<Self, String> {
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

    fn populate_hooks_scripts(hooks: &mut Hooks) -> Result<(), String> {
        let Ok(entries) = hooks.root.read_dir() else {
            return Err(format!(
                "{fatal}: Could not read root directory for hooks.",
                fatal = ui::Color::error("fatal")
            ));
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

            let Some(file_prefix) = entry.file_prefix() else {
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
        hooks.set_env_var("DEEZ_OS", env::consts::OS);
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
    pub fn pre_sync(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.pre_sync)
    }

    /// Run "post-sync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_sync(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.post_sync)
    }

    /// Run "pre-rsync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_rsync(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.pre_rsync)
    }

    /// Run "post-rsync" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_rsync(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.post_rsync)
    }

    /// Run "pre-link" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_link(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.pre_link)
    }

    /// Run "post-link" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_link(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.post_link)
    }

    /// Run "pre-status" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_status(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.pre_status)
    }

    /// Run "post-status" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_status(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.post_status)
    }

    /// Run "pre-diff" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_diff(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.pre_diff)
    }

    /// Run "post-diff" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_diff(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.post_diff)
    }

    /// Run "pre-clean" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn pre_clean(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.pre_clean)
    }

    /// Run "post-clean" hooks.
    ///
    /// Returns the number of hooks that ran.
    ///
    /// # Errors
    ///
    /// Returns an error if the `sh` executable cannot be found.
    pub fn post_clean(&self) -> Result<usize, String> {
        self.run_hooks(&self.scripts.post_clean)
    }

    fn run_hooks(&self, hooks: &[PathBuf]) -> Result<usize, String> {
        for hook in hooks {
            self.run_hook(hook)?;
        }
        Ok(hooks.len())
    }

    fn run_hook(&self, hook: &Path) -> Result<(), String> {
        // `root` cannot be an empty `Path`.
        debug_assert!(!self.root.to_str().is_some_and(str::is_empty));

        if self.is_verbose {
            println!("hook: {}", hook.display());
        }

        let hook_file = self.root.join(hook);

        let status = process::Command::new("sh")
            .arg("-c")
            .arg(&hook_file) // Always a path (`root` is never empty).
            .envs(&self.envs)
            .current_dir(self.root)
            .status();

        match status {
            Err(_) => Err(format!(
                "{fatal}: Could not find the 'sh' executable.",
                fatal = ui::Color::error("fatal")
            )),
            Ok(status) => {
                if status.success() {
                    Ok(())
                } else {
                    // We can't differentiate (based solely on the exit
                    // code) whether the hook failed, or `sh` failed
                    // (i.e., did the hook exit 1, or did `sh` exit 1?).
                    // The main reason `sh` can fail is if the hook is
                    // not executable. That's what we check here, and if
                    // it's the case, we provide a more useful error
                    // message than "Execution aborted by '<hook>'".
                    #[cfg(unix)]
                    {
                        use std::fs::File;
                        use std::os::unix::fs::PermissionsExt;

                        if let Ok(metadata) = File::open(&hook_file).and_then(|f| f.metadata())
                            && metadata.permissions().mode() & 0o111 == 0
                        {
                            return Err(format!(
                                "{error}: '{}' is not executable.\nConsider running 'chmod +x {}'.",
                                hook.display(),
                                hook.display(),
                                error = ui::Color::error("error")
                            ));
                        }
                        // Else don't bother. We're in bonus territory.
                    }

                    Err(format!(
                        "{abort}: Execution aborted by '{}'.",
                        hook.display(),
                        abort = ui::Color::error("abort")
                    ))
                }
            }
        }
    }

    /// List of hooks, grouped by type, and in execution order.
    ///
    /// Hooks are already sorted in lexicographic order, that also
    /// determines the order of execution.
    #[must_use]
    pub fn list(&self) -> Vec<Cow<'_, str>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn empty_scripts() -> Scripts {
        Scripts {
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
        }
    }

    fn hooks_with(scripts: Scripts) -> Hooks<'static> {
        Hooks {
            root: Path::new("/root"),
            home: Path::new("/home"),
            is_verbose: false,
            envs: HashMap::new(),
            scripts,
        }
    }

    #[test]
    fn is_hook_detects_hooks_and_non_hooks() {
        assert!(is_hook(Path::new("pre-sync.sh")));
        assert!(is_hook(Path::new("post-clean")));
        assert!(is_hook(Path::new("pre-diff.001.py")));

        assert!(!is_hook(Path::new("regular.txt")));
        assert!(!is_hook(Path::new(".gitconfig")));
        // A path with no file prefix falls through to `false`.
        assert!(!is_hook(Path::new("")));
    }

    #[test]
    fn hooks_are_sorted_by_file_name() {
        let mut scripts = empty_scripts();
        // Deliberately out of order.
        scripts.pre_sync = vec![
            PathBuf::from("pre-sync.sh"),
            PathBuf::from("pre-sync.002.sh"),
            PathBuf::from("pre-sync.py"),
            PathBuf::from("pre-sync.001.py"),
        ];

        let mut hooks = hooks_with(scripts);
        Hooks::sort_hooks_scripts_by_file_name(&mut hooks);

        assert_eq!(
            hooks.scripts.pre_sync,
            vec![
                PathBuf::from("pre-sync.001.py"),
                PathBuf::from("pre-sync.002.sh"),
                PathBuf::from("pre-sync.py"),
                PathBuf::from("pre-sync.sh"),
            ]
        );
    }

    #[test]
    fn list_groups_hooks_in_execution_order() {
        // Populate a few non-adjacent groups to prove ordering follows
        // the `HOOKS` grouping, not insertion order.
        let mut scripts = empty_scripts();
        scripts.post_clean = vec![PathBuf::from("post-clean.sh")];
        scripts.pre_sync = vec![PathBuf::from("pre-sync.sh")];
        scripts.post_sync = vec![PathBuf::from("post-sync.sh")];
        scripts.pre_clean = vec![PathBuf::from("pre-clean.sh")];

        let hooks = hooks_with(scripts);
        let list: Vec<String> = hooks.list().iter().map(ToString::to_string).collect();

        assert_eq!(
            list,
            vec![
                "pre-sync.sh".to_string(),
                "post-sync.sh".to_string(),
                "pre-clean.sh".to_string(),
                "post-clean.sh".to_string(),
            ]
        );
    }
}
