use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process;

use deezconfigs::hooks::Hooks;
use deezconfigs::ui;

/// Resolve config root for a given path.
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
/// inadvertently mess up his home directory by syncing the wrong root.
///
/// # Note
///
/// The check can be disabled by setting `do_check` to `false`. This is
/// _not_ a user-facing option. It is used internally by non-fs-altering
/// commands that don't need it, such as `status` for instance.
pub fn resolve_config_root(root: Option<&String>, do_check: bool) -> Result<PathBuf, i32> {
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
Please provide a root directory as argument.
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
This is a security feature. `{bin}` doesn't want to mess up your home
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
/// `/tmp` on Unix) under the name `deez-<pid>-<uuid>`.
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
    let pid = std::process::id();
    let uuid = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("current time > Unix epoch")
        .as_millis();
    let clone_path = env::temp_dir().join(format!("deez-{pid}-{uuid}"));

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

    // TODO: Clones are never cleanup up. This is not a big issue since
    // we clone into a temporary directory, but it's sloppy. We could
    // register the path into a global "cleanup queue" that runs at the
    // end. Side question: should we expose a `DEEZ_TMP` variable to
    // hooks (could be cleaned up the same way). Or always have a global
    // temporary dir available (`LazyLock`), expose it to hooks, and
    // adapt the clone to clone into a sub-directory there.
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
        if !is_sub_root_safe(sub_root) {
            eprintln!(
                "{fatal}: Sub-root must not contain '..' components: '{sub_root}'.",
                fatal = ui::Color::error("fatal")
            );
            return Err(1);
        }

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

/// Resolve a local config root, ask for confirmation if it has no
/// `.deez` marker, then run `git pull`. Remote roots are rejected.
pub fn resolve_and_pull_config_root(root: Option<&String>) -> Result<PathBuf, i32> {
    if is_git_remote_uri(root) {
        eprintln!(
            "{fatal}: '--pull' only works with local config roots.",
            fatal = ui::Color::error("fatal")
        );
        return Err(2);
    }
    let root = resolve_config_root(root, true)?;
    run_git_pull_in_root(&root)?;
    Ok(root)
}

/// Run `git pull` inside an already-resolved config root.
///
/// This calls Git directly because `cmd::run()` takes its root from
/// `DEEZ_ROOT`; it cannot accept the path resolved for this command.
fn run_git_pull_in_root(root: &Path) -> Result<(), i32> {
    let status = process::Command::new("git")
        .current_dir(root)
        .arg("pull")
        .status();

    propagate_git_status(status, "pull")
}

/// Run `git fetch` inside an already-resolved config root.
///
/// Nothing is passed to Git, and nothing is captured from it. Progress
/// and errors go straight to the user, in Git's own words.
///
/// # Errors
///
/// Errors if Git cannot be run, or with Git's own exit code if the
/// fetch itself fails (e.g., no network, no such remote).
pub fn run_git_fetch_in_root(root: &Path) -> Result<(), i32> {
    let status = process::Command::new("git")
        .current_dir(root)
        .arg("fetch")
        .status();

    propagate_git_status(status, "fetch")
}

/// Show the changes between `HEAD` and its upstream branch.
///
/// By default, this shows the _incoming_ changes: what the upstream has
/// that `HEAD` doesn't. `reversed` shows the _outgoing_ ones instead.
///
/// Both ranges use three dots, so they start at the merge base. This is
/// what makes `--incoming` leave out your own local commits.
///
/// Output is left entirely to Git: its own colors, its own pager, and
/// its own error messages (e.g., if there is no upstream branch, or if
/// the root is not a Git repository).
///
/// # Errors
///
/// Errors if Git cannot be run, or with Git's own exit code if the diff
/// itself fails.
pub fn show_git_diff_against_upstream(root: &Path, reversed: bool) -> Result<(), i32> {
    let range = if reversed {
        "@{u}...HEAD"
    } else {
        "HEAD...@{u}"
    };

    let mut command = process::Command::new("git");
    command.current_dir(root).arg("diff");

    // Git does not support `NO_COLOR`, it only knows about `color.ui`
    // and friends. But `deez` does support it, everywhere else. So we
    // forward the user's preference, rather than silently dropping it.
    if *ui::color::NO_COLOR {
        command.arg("--no-color");
    }

    // `--` so that Git doesn't mistake the range for a path.
    command.arg(range).arg("--");

    propagate_git_status(command.status(), "diff")
}

/// Turn the result of a `git` invocation into a `deez` exit code.
///
/// Git's exit code is propagated as-is, because Git will have explained
/// itself already. The one case Git cannot report is not being there at
/// all.
fn propagate_git_status(
    status: Result<process::ExitStatus, std::io::Error>,
    command: &str,
) -> Result<(), i32> {
    match status {
        Ok(status) => match status.code() {
            Some(0) => Ok(()),
            Some(code) => Err(code),
            None => Err(1),
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            eprint!(
                "\
{fatal}: Could not run 'git {command}'.
Did not find the 'git' executable. Please ensure Git is properly
installed on your machine.
",
                fatal = ui::Color::error("fatal")
            );
            Err(1)
        }
        Err(err) => {
            eprintln!(
                "{fatal}: Could not run 'git {command}': {err}",
                fatal = ui::Color::error("fatal")
            );
            Err(1)
        }
    }
}

/// Whether the sub-root is safe (no `..` directory traversal).
fn is_sub_root_safe(sub_root: &str) -> bool {
    !Path::new(sub_root)
        .components()
        .any(|c| c == Component::ParentDir)
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

/// Get the user's home directory.
///
/// The home directory is read from `HOME` environment variable.
pub fn get_home_directory() -> Result<PathBuf, i32> {
    if let Some(home_directory) = std::env::home_dir() {
        Ok(home_directory)
    } else {
        eprintln!(
            "{fatal}: Could not read home directory from environment.",
            fatal = ui::Color::error("fatal")
        );
        Err(1)
    }
}

/// Helper function to instantiate [`Hooks`] from a command, or error.
pub fn get_hooks_for_command<'a>(
    root: &'a impl AsRef<Path>,
    home: &'a impl AsRef<Path>,
    verbose: bool,
) -> Result<Hooks<'a>, i32> {
    match Hooks::for_command(root.as_ref(), home.as_ref(), verbose) {
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
    fn test_is_sub_root_safe() {
        // Safe.
        assert!(is_sub_root_safe("foo/bar"));
        assert!(is_sub_root_safe("foo"));
        assert!(is_sub_root_safe("foo/bar/baz"));

        assert!(is_sub_root_safe(".hidden"));
        assert!(is_sub_root_safe("foo/.hidden/bar"));
        assert!(is_sub_root_safe("..tricky"));
        assert!(is_sub_root_safe("tri..cky"));
        assert!(is_sub_root_safe("tricky.."));
        assert!(is_sub_root_safe("..."));
        assert!(is_sub_root_safe("foo/..tricky/bar"));
        assert!(is_sub_root_safe("foo/tri..cky/bar"));
        assert!(is_sub_root_safe("foo/.../bar"));

        // Unsafe.
        assert!(!is_sub_root_safe(".."));
        assert!(!is_sub_root_safe("../foo"));
        assert!(!is_sub_root_safe("foo/.."));
        assert!(!is_sub_root_safe("foo/../bar"));
        assert!(!is_sub_root_safe("foo/../../etc"));
        assert!(!is_sub_root_safe("../../etc/passwd"));
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
