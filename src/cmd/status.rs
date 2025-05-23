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
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use deezconfigs::{ui, utils, walk};

use super::common::{
    determine_config_root, get_config_root_from_git, get_home_directory, get_hooks_for_command,
    is_git_remote_uri, run_hooks,
};

#[derive(Debug, Eq, PartialEq)]
enum State {
    InSync,
    Modified,
    Missing,
}

#[derive(Debug, Eq, PartialEq)]
struct Status {
    file: String,
    state: State,
    is_symlinked: bool,
}

impl PartialOrd for Status {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Status {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.file.cmp(&other.file)
    }
}

/// Collect synchronization statuses for config files.
///
/// 1. Collect all files in `configs`.
/// 2. Compare with files in `$HOME` to get status:
///    - In Sync (equal).
///    - Modified (not equal).
///    - Missing (not yet copied).
pub fn status(root: Option<&String>, verbose: bool) -> Result<(), i32> {
    let root = if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        determine_config_root(root, false)?
    };
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_status())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let statuses = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, |p| {
        debug_assert!(!p.is_dir());

        let source = root.join(p);
        let destination = home.join(p);

        let status = Status {
            file: p.to_string_lossy().to_string(),
            state: if destination.is_file() {
                match are_files_equal(&source, &destination) {
                    Ok(equal) => {
                        if equal {
                            State::InSync
                        } else {
                            State::Modified
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "{error}: Could not compare '{}': {err}.",
                            source.display(),
                            error = ui::Color::error("error"),
                        );
                        return;
                    }
                }
            } else {
                State::Missing
            },
            is_symlinked: destination.is_symlink(),
        };

        match statuses.lock() {
            Ok(mut statuses) => {
                statuses.push(status);
                // Release the lock ASAP.
                drop(statuses);
            }
            Err(err) => {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not acquire lock: {err}.",
                    error = ui::Color::error("error")
                );
                #[allow(clippy::needless_return)] // Keep this one explicit.
                return;
            }
        }
    });

    let mut statuses = Arc::try_unwrap(statuses)
        .expect("processing is over, we're back to a single thread.")
        .into_inner()
        .unwrap();
    // Do not use `sort_unstable()` because the files are likely
    // _partially_ sorted, in which case stable sort is faster,
    // as per the docs.
    statuses.sort();

    print_file_statuses(&statuses);
    print_hooks(&hooks.list());
    print_summary_line(&statuses);

    nb_hooks_ran += run_hooks(|| hooks.post_status())?;

    ui::print_hooks_summary(nb_hooks_ran);

    if nb_errors.into_inner() > 0 {
        Err(1)
    } else {
        Ok(())
    }
}

fn are_files_equal(a: &Path, b: &Path) -> Result<bool, std::io::Error> {
    // Possible improvements if this is a bottleneck:
    //  - Compare _streaming_ bytes, to cater for big files.
    //  - Compare hashes (e.g., xxHashes) if we do the streaming.
    //    Streaming is slower because you have to jump back-and-forth
    //    between files.

    // 1. Compare by file size (quick).
    if fs::metadata(a)?.len() != fs::metadata(b)?.len() {
        return Ok(false);
    }

    // 2. Compare contents (slow; as raw bytes to avoid UTF-8 overhead).
    thread_local! {
        static BUFFERS: RefCell<(Vec<u8>, Vec<u8>)> = RefCell::new(
            // 64 Kb should be plenty for the majority of config files.
            (Vec::with_capacity(65_536), Vec::with_capacity(65_536))
        );
    }

    BUFFERS.with_borrow_mut(|(a_buf, b_buf)| {
        utils::read_to_bytes_buffer(a_buf, a)?;
        utils::read_to_bytes_buffer(b_buf, b)?;

        Ok(a_buf == b_buf)
    })
}

fn print_file_statuses(statuses: &[Status]) {
    let summary = statuses
        .iter()
        .map(|s| {
            format!(
                "  {}  {}{}",
                match &s.state {
                    State::InSync => ui::Color::in_sync("S"),
                    State::Modified => ui::Color::modified("M"),
                    State::Missing => ui::Color::missing("!"),
                },
                s.file,
                if s.is_symlinked {
                    ui::Color::symlink("@")
                } else {
                    ui::Color::none("")
                },
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    println!("Files\n{summary}");
}

fn print_hooks(hooks: &[Cow<str>]) {
    if hooks.is_empty() {
        return;
    }
    let summary = hooks
        .iter()
        .map(|h| format!("  {h}"))
        .collect::<Vec<String>>()
        .join("\n");

    println!("Hooks\n{summary}");
}

fn print_summary_line(statuses: &[Status]) {
    let (mut nb_in_sync, mut nb_modified, mut nb_missing) = (0, 0, 0);
    for s in statuses {
        match &s.state {
            State::InSync => nb_in_sync += 1,
            State::Modified => nb_modified += 1,
            State::Missing => nb_missing += 1,
        }
    }

    println!("{nb_in_sync} in sync, {nb_modified} modified, {nb_missing} missing.");
}
