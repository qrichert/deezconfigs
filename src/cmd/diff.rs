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
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use lessify::Pager;

use deezconfigs::{ui, walk};

use super::common::{
    determine_config_root, get_config_root_from_git, get_home_directory, get_hooks_for_command,
    is_git_remote_uri, run_hooks,
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

/// Diff files in Root and Home.
///
/// 1. Collect all files in `configs`.
/// 2. Diff with files in `$HOME`.
pub fn diff(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        determine_config_root(root, false)?
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
            let diff = match diff_files(&source, &destination) {
                Ok(diff) => diff,
                Err(err) => {
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("error: Could not compare '{}': {err}.", p.display());
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
                diff: String::from("! File does not exist in Home.\n! Skipping..."),
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
                eprintln!("error: Could not acquire lock: {err}.");
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

    Ok(())
}

fn diff_files(before: &Path, after: &Path) -> Result<Option<String>, std::io::Error> {
    use imara_diff::intern::InternedInput;
    use imara_diff::{Algorithm, UnifiedDiffBuilder, diff};

    // TODO: Thread-local buffer.
    let before = fs::read_to_string(before)?;
    let after = fs::read_to_string(after)?;

    let input = InternedInput::new(before.as_str(), after.as_str());
    let diff = diff(
        Algorithm::Histogram,
        &input,
        UnifiedDiffBuilder::new(&input),
    );

    if diff.is_empty() {
        return Ok(None);
    }

    Ok(Some(diff))
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
