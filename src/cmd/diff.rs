use std::borrow::Cow;
use std::cell::RefCell;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use lessify::Pager;

use deezconfigs::{ui, utils, walk};

use super::common::{
    get_config_root_from_git, get_home_directory, get_hooks_for_command, is_git_remote_uri,
    resolve_and_pull_config_root, resolve_config_root, run_git_fetch_in_root, run_hooks,
    show_git_diff_against_upstream,
};

#[derive(Debug, Eq, PartialEq)]
struct Diff {
    file: String,
    diff: String,
}

impl PartialOrd for Diff {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Diff {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.file.cmp(&other.file)
    }
}

/// Diff files in root and home.
///
/// 1. Collect all files in `configs`.
/// 2. Diff with files in `$HOME`.
pub fn diff(
    root: Option<&String>,
    verbose: bool,
    pull_before_command: bool,
    reversed: bool,
) -> Result<(), i32> {
    let root = if pull_before_command {
        resolve_and_pull_config_root(root)?
    } else if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        resolve_config_root(root, false)?
    };
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_diff())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let diffs = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, |p| {
        debug_assert!(!p.is_dir());

        let source = root.join(p);
        let destination = home.join(p);

        let diff = if destination.is_file() {
            let diff = if reversed {
                diff_files(&source, &destination)
            } else {
                diff_files(&destination, &source)
            };
            let diff = match diff {
                Ok(diff) => diff,
                Err(err) => {
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "{error}: Could not compare '{}': {err}.",
                        p.display(),
                        error = ui::Color::error("error"),
                    );
                    return;
                }
            };

            let Some(diff) = diff else {
                return;
            };

            Diff {
                file: p.to_string_lossy().to_string(),
                diff,
            }
        } else {
            Diff {
                file: p.to_string_lossy().to_string(),
                diff: String::from("! File does not exist in home.\n! Skipping..."),
            }
        };

        match diffs.lock() {
            Ok(mut diffs) => {
                diffs.push(diff);
                // Release the lock ASAP.
                drop(diffs);
            }
            Err(err) => {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not acquire lock: {err}.",
                    error = ui::Color::error("error"),
                );
                #[allow(clippy::needless_return)] // Keep this one explicit.
                return;
            }
        }
    });

    let mut diffs = Arc::try_unwrap(diffs)
        .expect("processing is over, we're back to a single thread.")
        .into_inner()
        .unwrap();
    // Do not use `sort_unstable()` because the files are likely
    // _partially_ sorted, in which case stable sort is faster,
    // as per the docs.
    diffs.sort();

    // For this command, run hooks _before_ printing, because the output
    // is likely paged.
    nb_hooks_ran += run_hooks(|| hooks.post_diff())?;

    let nb_errors = nb_errors.into_inner();

    if nb_errors == 0 {
        if diffs.is_empty() {
            println!("Home is in sync.");
        } else {
            print_file_diffs(&diffs);
        }
    }

    ui::print_hooks_summary(nb_hooks_ran);

    if nb_errors > 0 { Err(1) } else { Ok(()) }
}

/// Show incoming changes from the Git remote.
///
/// Fetches, then hands the patch straight to Git. Nothing is captured
/// or re-rendered: this is a shortcut for
/// `git fetch; git diff HEAD...@{u}`, not a comparison between the root
/// and the home.
///
/// 1. `git fetch` in the config root.
/// 2. `git diff` between `HEAD` and its upstream.
pub fn diff_incoming(
    root: Option<&String>,
    verbose: bool,
    pull_before_command: bool,
    reversed: bool,
) -> Result<(), i32> {
    if is_git_remote_uri(root) {
        eprintln!(
            "{fatal}: '--incoming' only works with local config roots.",
            fatal = ui::Color::error("fatal")
        );
        return Err(2);
    }
    if pull_before_command {
        eprint!(
            "\
{fatal}: '--incoming' cannot be combined with '--pull'.
`--pull` merges the incoming changes, leaving nothing to show.
",
            fatal = ui::Color::error("fatal")
        );
        return Err(2);
    }

    // Fetching only writes to `.git`, never to the home, so there is
    // nothing for the `.deez` check to protect against here.
    let root = resolve_config_root(root, false)?;

    // Fetch before the hooks, like `--pull` does. A failure to reach
    // the remote should not run anything.
    run_git_fetch_in_root(&root)?;

    // Only needed to build the hooks' environment.
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_diff())?;

    show_git_diff_against_upstream(&root, reversed)?;

    // Contrary to `diff()`, hooks run _after_ printing here. Git pages
    // its own output, and waits for the pager to exit before returning,
    // so by now the user is done reading.
    nb_hooks_ran += run_hooks(|| hooks.post_diff())?;

    ui::print_hooks_summary(nb_hooks_ran);

    Ok(())
}

fn diff_files(before: &Path, after: &Path) -> Result<Option<String>, std::io::Error> {
    use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};

    thread_local! {
        static BUFFERS: RefCell<(String, String)> = RefCell::new(
            // 64 Kb should be plenty for the majority of config files.
            (String::with_capacity(65_536), String::with_capacity(65_536))
        );
    }

    BUFFERS.with_borrow_mut(|(before_buf, after_buf)| {
        utils::read_to_string_buffer(before_buf, before)?;
        utils::read_to_string_buffer(after_buf, after)?;

        let input = InternedInput::new(before_buf.as_str(), after_buf.as_str());
        let mut diff = Diff::compute(Algorithm::Histogram, &input);
        diff.postprocess_lines(&input);

        if diff.hunks().next().is_none() {
            return Ok(None);
        }

        let diff = diff
            .unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string();

        Ok(Some(diff))
    })
}

fn print_file_diffs(diffs: &[Diff]) {
    let diffs = diffs
        .iter()
        .map(|d| {
            format!(
                "{}\n{}\n",
                ui::Color::file_name(&d.file),
                d.diff
                    .lines()
                    .map(|l| {
                        match l.chars().next() {
                            Some('+') => ui::Color::in_sync(l),
                            Some('-' | '!') => ui::Color::missing(l),
                            Some('@') => ui::Color::line_range(l),
                            _ => Cow::Borrowed(l),
                        }
                    })
                    .collect::<Vec<Cow<str>>>()
                    .join("\n")
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    Pager::page_or_print(&diffs);
}
