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

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use deezconfigs::hooks::Hooks;
use deezconfigs::ui;

/// Determine config root for a given path.
///
/// The root is either provided by the user, or we use a heuristic to
/// find an appropriate one to use:
///
/// 1. Get current working directory (cwd).
/// 2. If cwd is not a config root, look into its parents.
/// 3. If none of its parents is a config route, keep using cwd.
///
/// In any case, the selected root is then checked to ensure it is a
/// config root (contains a `.deez` file). If not, we let the user
/// choose to use it anyway, or to abort.
///
/// This check is essential to make, because otherwise the user may
/// inadvertently mess up his Home directory by syncing the wrong root.
///
/// # Note
///
/// The check can be disabled by setting `do_check` to `false`. This is
/// _not_ a user-facing option. It is used internally by non-fs-altering
/// commands that don't need it, such as `status` for instance.
pub fn determine_config_root(root: Option<&String>, do_check: bool) -> Result<PathBuf, i32> {
    // Given.
    let root = if let Some(root) = get_config_root_from_args(root) {
        root
    // Not given.
    } else {
        // Try current dir.
        let mut default = get_default_config_root()?;
        // If not, look inside parents.
        if !is_a_config_root(&default) {
            if let Some(parent) = find_config_root_in_parents(&default) {
                default = parent.to_path_buf();
            // If not, try `DEEZ_ROOT`.
            } else if let Some(root) = get_config_root_from_config() {
                default = root;
            }
            // Else, let current dir fail.
        }
        default
    };
    ensure_root_exists(&root)?;
    if do_check {
        ensure_root_is_a_config_root(&root)?;
    }
    Ok(root)
}

fn get_config_root_from_args(root: Option<&String>) -> Option<PathBuf> {
    if let Some(root) = root
        && !root.is_empty()
    {
        Some(PathBuf::from(root))
    } else {
        None
    }
}

pub fn get_config_root_from_config() -> Option<PathBuf> {
    if let Some(root) = env::var("DEEZ_ROOT").ok()
        && !root.is_empty()
    {
        Some(PathBuf::from(root))
    } else {
        None
    }
}

fn get_default_config_root() -> Result<PathBuf, i32> {
    let Ok(root) = env::current_dir() else {
        eprint!(
            "\
{fatal}: Could not determine current working directory.
Please provide a Root directory as argument.
",
            fatal = ui::Color::error("fatal")
        );
        return Err(1);
    };
    Ok(root)
}

fn find_config_root_in_parents(root: &Path) -> Option<&Path> {
    const DEPTH_LIMIT: usize = 20;
    // `skip()` self.
    for (i, candidate) in root.ancestors().skip(1).enumerate() {
        if is_a_config_root(candidate) {
            return Some(candidate);
        }
        if i == DEPTH_LIMIT {
            break;
        }
    }
    None
}

pub fn ensure_root_exists(root: &Path) -> Result<(), i32> {
    if root.is_dir() {
        return Ok(());
    }

    eprintln!(
        "{fatal}: Root must be a valid directory.",
        fatal = ui::Color::error("fatal")
    );

    // Be specific.
    if root.is_file() {
        eprintln!("'{}' is a file.", root.display());
    } else if !root.exists() {
        if root.to_str().is_some_and(str::is_empty) {
            eprintln!("No path provided.");
        } else {
            eprintln!("'{}' does not exist.", root.display());
        }
    }

    Err(1)
}

/// Ensure `root` holds config and is not a random directory.
///
/// To be a config root, the directory must contain a `.deez` file, or
/// the user must give confirmation.
fn ensure_root_is_a_config_root(root: &Path) -> Result<(), i32> {
    if is_a_config_root(root) {
        return Ok(());
    }

    eprint!(
        "\
{warning}: `root` is not a configuration root.

To make it a configuration root, create a `.deez` file inside of it.
This is a security feature. `{bin}` doesn't want to mess up your Home
directory if you run it in the wrong root.

Selected root: '{}'.

",
        root.display(),
        warning = ui::Color::warning("warning"),
        bin = env!("CARGO_BIN_NAME"),
    );

    if ui::ask_confirmation_with_prompt("Proceed?") {
        println!();
        return Ok(());
    }

    eprintln!("Aborting.");

    Err(2)
}

fn is_a_config_root(root: &Path) -> bool {
    root.join(".deez").is_file()
}

/// Detect if provided root is a Git remote.
pub fn is_git_remote_uri(root: Option<&String>) -> bool {
    root.is_some_and(|root| {
        ["git:", "ssh://", "git@", "https://", "http://", "gh:"]
            .iter()
            .any(|prefix| root.starts_with(prefix))
    })
}

/// Clone Git repository and return its path.
///
/// The repository is cloned to the system's temporary directory (e.g.,
/// `/tmp` on Unix) under the name `deez-<uuid>`.
///
/// # Errors
///
/// Errors if the temporary directory cannot be written to, or if
/// `git clone` fails.
///
/// `git clone` can fail either because the Git binary cannot be found,
/// or because the command itself fails (e.g., due to network issues,
/// access rights, etc.).
pub fn get_config_root_from_git(uri: &str, verbose: bool) -> Result<PathBuf, i32> {
    let uri = if let Some(uri) = uri.strip_prefix("git:") {
        uri.to_string()
    } else if let Some(uri) = uri.strip_prefix("gh:") {
        format!("git@github.com:{uri}")
    } else {
        uri.to_string()
    };

    // Extract potential sub root.
    // git@github.com/qrichert/configs[sub/root]
    let (uri, sub_root) = extract_sub_root(&uri);

    // Yes, I know. Not a solid UUID, I should use a crate, etc.
    let uuid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time > Unix epoch")
        .as_millis()
        .to_string();
    let clone_path = env::temp_dir().join(format!("deez-{uuid}"));

    if clone_path.is_dir() && fs::remove_dir_all(&clone_path).is_err() {
        eprint!(
            "\
{fatal}: Could not clone the configuration repository.
The target directory already exists and could not be deleted.
",
            fatal = ui::Color::error("fatal")
        );
        return Err(1);
    }

    println!("Fetching config files remotely...");

    let mut command = process::Command::new("git");
    command
        .env("LANG", "en_US.UTF-8")
        .arg("clone")
        .arg("--single-branch")
        .arg("--depth=1")
        .arg("--no-tags")
        .arg(uri)
        .arg(&clone_path);

    let status = if verbose {
        command.status().ok()
    } else {
        command.arg("--quiet");
        command.output().ok().map(|out| out.status)
    };

    if let Some(status) = status {
        if !status.success() {
            eprintln!(
                "{fatal}: Could not clone the configuration repository.",
                fatal = ui::Color::error("fatal")
            );
            if !verbose {
                eprintln!("Retry with `--verbose` for additional detail.");
            }
            return Err(1);
        }
    } else {
        eprint!(
            "\
{fatal}: Could not clone the configuration repository.
Did not find the 'git' executable. Please ensure Git is properly
installed on your machine.
",
            fatal = ui::Color::error("fatal")
        );
        return Err(1);
    }

    println!("Done.");

    if let Some(sub_root) = sub_root {
        let clone_path = clone_path.join(sub_root);

        if !clone_path.is_dir() {
            eprintln!(
                "{fatal}: Cannot find sub-root inside Git repository: '{sub_root}'.",
                fatal = ui::Color::error("fatal")
            );
            return Err(1);
        }

        Ok(clone_path)
    } else {
        Ok(clone_path)
    }
}

/// Extract sub-root from Git remote URL.
///
/// Sub-roots are defined by appending `[sub/root]` to the remote URL.
/// For example: `git@github.com/qrichert/configs[sub/root]`
///
/// # Note
///
/// Sub-roots are returned without leading slashes (`/`), to force them
/// to be relative (to the root). An absolute sub-root would replace the
/// base path if `join()`ed; not what we want.
///
/// Sub-roots are also returned trimmed (no whitespace around).
///
/// Sub-roots evaluate to `None` if empty.
fn extract_sub_root(uri: &str) -> (&str, Option<&str>) {
    if let Some((uri, sub_root)) = uri.rsplit_once('[')
        && sub_root.ends_with(']')
    {
        let sub_root = sub_root
            .strip_suffix(']')
            .expect("we checked that it ends with ']'")
            .trim()
            // No leading slash! It would override paths on `join()`.
            .trim_start_matches('/');
        if sub_root.is_empty() {
            (uri, None)
        } else {
            (uri, Some(sub_root))
        }
    } else {
        (uri, None)
    }
}

/// Get the user's Home directory.
///
/// The Home directory is read from `HOME` environment variable.
pub fn get_home_directory() -> Result<PathBuf, i32> {
    if let Some(home_directory) = std::env::home_dir() {
        Ok(home_directory)
    } else {
        eprintln!(
            "{fatal}: Could not read Home directory from environment.",
            fatal = ui::Color::error("fatal")
        );
        Err(1)
    }
}

/// Helper function to instantiate [`Hooks`] from a command, or error.
pub fn get_hooks_for_command<'a>(
    root: &'a Path,
    home: &'a Path,
    verbose: bool,
) -> Result<Hooks<'a>, i32> {
    match Hooks::for_command(root, home, verbose) {
        Ok(hooks) => Ok(hooks),
        Err(err) => {
            eprintln!("{err}");
            Err(1)
        }
    }
}

/// Helper function to run a group of hooks, or error.
pub fn run_hooks(hooks: impl Fn() -> Result<usize, String>) -> Result<usize, i32> {
    match hooks() {
        Ok(nb_hooks) => Ok(nb_hooks),
        Err(err) => {
            eprintln!("{err}");
            Err(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_remote_uri() {
        fn is_git_uri(uri: &'static str) -> bool {
            is_git_remote_uri(Some(&uri.to_string()))
        }
        assert!(is_git_uri("git:../configs"));
        assert!(is_git_uri("git:~/Developer/configs"));
        assert!(is_git_uri("ssh://misc/home/misc/configs"));
        assert!(is_git_uri("git@github.com:qrichert/configs.git"));
        assert!(is_git_uri("https://github.com/qrichert/configs.git"));
        assert!(is_git_uri("http://github.com/qrichert/configs.git"));
        assert!(is_git_uri("gh:qrichert/configs.git"));
    }

    #[test]
    fn test_extract_sub_root() {
        assert_eq!(
            extract_sub_root("../configs[foo/bar]"),
            ("../configs", Some("foo/bar"))
        );
        assert_eq!(
            extract_sub_root("~/Developer/configs[/foo/bar]"),
            ("~/Developer/configs", Some("foo/bar"))
        );
        assert_eq!(
            extract_sub_root("ssh://misc/home/[misc]/configs[ /foo/bar ]"),
            ("ssh://misc/home/[misc]/configs", Some("foo/bar"))
        );
        assert_eq!(
            extract_sub_root("git@github.com:qrichert/configs.git"),
            ("git@github.com:qrichert/configs.git", None)
        );
        assert_eq!(
            extract_sub_root("https://github.com/qrichert/configs.git[]"),
            ("https://github.com/qrichert/configs.git", None)
        );
        assert_eq!(
            extract_sub_root("http://github.com/qrichert/configs.git[ ]"),
            ("http://github.com/qrichert/configs.git", None)
        );
        assert_eq!(
            extract_sub_root("qrichert/configs.git[ / ]"),
            ("qrichert/configs.git", None)
        );
    }
}
