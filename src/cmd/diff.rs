use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use lessify::Pager;

use deezconfigs::pathspec::PathSpec;
use deezconfigs::{ui, utils, walk};

use super::common::{
    get_config_root_from_git, get_home_directory, get_hooks_for_command, is_git_remote_uri,
    resolve_and_pull_config_root, resolve_config_root, run_git_fetch_in_root, run_hooks,
    show_git_diff_against_upstream,
};

#[derive(Debug, Eq, PartialEq)]
struct Diff {
    file: String,
    diff: FileDiff,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct FileDiff {
    text: String,
    changes: Vec<ChangeRun>,
}

#[derive(Debug, Eq, PartialEq)]
struct ChangeRun {
    kind: ChangeKind,
    lines: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChangeKind {
    Removed,
    Added,
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
    pathspec: &PathSpec,
) -> Result<(), i32> {
    let root = if pull_before_command {
        resolve_and_pull_config_root(root)?.into()
    } else if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        resolve_config_root(root, false)?.into()
    };
    let root: &Path = root.as_ref();
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_diff())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let diffs = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_errors = AtomicUsize::new(0);
    let nb_processed = AtomicUsize::new(0);

    walk::find_files_recursively(root, pathspec, |p| {
        debug_assert!(!p.is_dir());

        nb_processed.fetch_add(1, Ordering::Relaxed);

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
                diff: FileDiff {
                    text: String::from("! File does not exist in home.\n! Skipping..."),
                    ..FileDiff::default()
                },
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
    let nb_processed = nb_processed.into_inner();

    if nb_errors == 0 {
        if !pathspec.is_empty() && nb_processed == 0 {
            // A filter was given but selected nothing; "in sync" would
            // be misleading here.
            println!("No files matched.");
        } else if diffs.is_empty() {
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
    pathspecs: &[String],
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

    show_git_diff_against_upstream(&root, reversed, pathspecs)?;

    // Contrary to `diff()`, hooks run _after_ printing here. Git pages
    // its own output, and waits for the pager to exit before returning,
    // so by now the user is done reading.
    nb_hooks_ran += run_hooks(|| hooks.post_diff())?;

    ui::print_hooks_summary(nb_hooks_ran);

    Ok(())
}

fn diff_files(before: &Path, after: &Path) -> Result<Option<FileDiff>, std::io::Error> {
    use imara_diff::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};

    let permissions_diff = diff_permissions(before, after)?;

    thread_local! {
        static BUFFERS: RefCell<(String, String)> = RefCell::new(
            // 64 Kb should be plenty for the majority of config files.
            (String::with_capacity(65_536), String::with_capacity(65_536))
        );
    }

    let contents_diff = BUFFERS.with_borrow_mut(
        |(before_buf, after_buf)| -> Result<Option<FileDiff>, std::io::Error> {
            utils::read_to_string_buffer(before_buf, before)?;
            utils::read_to_string_buffer(after_buf, after)?;

            let input = InternedInput::new(before_buf.as_str(), after_buf.as_str());
            let mut diff = Diff::compute(Algorithm::Histogram, &input);
            diff.postprocess_lines(&input);

            if diff.hunks().next().is_none() {
                return Ok(None);
            }

            let changes = if *ui::color::NO_COLOR {
                Vec::new()
            } else {
                collect_changed_line_runs(&diff, &input)
            };
            let text = diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string();

            Ok(Some(FileDiff { text, changes }))
        },
    )?;

    Ok(match (permissions_diff, contents_diff) {
        (None, None) => None,
        (Some(permissions), None) => Some(FileDiff {
            text: permissions,
            ..FileDiff::default()
        }),
        (None, Some(contents)) => Some(contents),
        (Some(permissions), Some(mut contents)) => {
            contents.text = format!("{permissions}\n{}", contents.text);
            Some(contents)
        }
    })
}

/// Collect changed-line runs from diff hunks for move detection.
///
/// A run contains the lines removed by one hunk or the lines added by
/// it. Each hunk contributes up to two runs: removed first, then added.
/// Unchanged lines are excluded.
fn collect_changed_line_runs(
    diff: &imara_diff::Diff,
    input: &imara_diff::InternedInput<&str>,
) -> Vec<ChangeRun> {
    let mut changes = Vec::new();
    for hunk in diff.hunks() {
        // Match imara's `BasicLineDiffPrinter` order, keeping original
        // line endings before the reusable input buffers are cleared.
        for (kind, tokens) in [
            (
                ChangeKind::Removed,
                &input.before[hunk.before.start as usize..hunk.before.end as usize],
            ),
            (
                ChangeKind::Added,
                &input.after[hunk.after.start as usize..hunk.after.end as usize],
            ),
        ] {
            if !tokens.is_empty() {
                changes.push(ChangeRun {
                    kind,
                    lines: tokens
                        .iter()
                        .map(|&token| input.interner[token].to_owned())
                        .collect(),
                });
            }
        }
    }
    changes
}

#[cfg(unix)]
fn diff_permissions(before: &Path, after: &Path) -> Result<Option<String>, std::io::Error> {
    use std::os::unix::fs::PermissionsExt;

    const PERMISSION_BITS: u32 = 0o7777;

    let before_mode = before.metadata()?.permissions().mode() & PERMISSION_BITS;
    let after_mode = after.metadata()?.permissions().mode() & PERMISSION_BITS;

    Ok((before_mode != after_mode)
        .then(|| format!("permissions {before_mode:04o} -> {after_mode:04o}")))
}

#[cfg(not(unix))]
fn diff_permissions(before: &Path, after: &Path) -> Result<Option<String>, std::io::Error> {
    let before_readonly = before.metadata()?.permissions().readonly();
    let after_readonly = after.metadata()?.permissions().readonly();

    Ok((before_readonly != after_readonly).then(|| {
        let before = if before_readonly {
            "read-only"
        } else {
            "writable"
        };
        let after = if after_readonly {
            "read-only"
        } else {
            "writable"
        };
        format!("permissions {before} -> {after}")
    }))
}

fn print_file_diffs(diffs: &[Diff]) {
    let moved = detect_moved_lines(diffs);
    let diffs = diffs
        .iter()
        .zip(&moved)
        .map(|(d, moved)| {
            let mut moved = moved.iter();
            format!(
                "{}\n{}\n",
                ui::Color::file_name(&d.file),
                d.diff
                    .text
                    .lines()
                    .map(|l| {
                        match l.chars().next() {
                            Some('+') => {
                                if moved.next().copied().unwrap_or(false) {
                                    ui::Color::moved_added(l)
                                } else {
                                    ui::Color::added(l)
                                }
                            }
                            Some('-') => {
                                if moved.next().copied().unwrap_or(false) {
                                    ui::Color::moved_removed(l)
                                } else {
                                    ui::Color::removed(l)
                                }
                            }
                            Some('!') => ui::Color::missing(l),
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

/// Classify all selected files together, allowing moves between files.
///
/// Match Git's `--color-moved=blocks` algorithm across all files.
fn detect_moved_lines(diffs: &[Diff]) -> Vec<Vec<bool>> {
    let runs = diffs
        .iter()
        .flat_map(|diff| &diff.diff.changes)
        .collect::<Vec<_>>();
    let mut lengths = matching_block_lengths(&runs).into_iter();
    diffs
        .iter()
        .map(|diff| {
            diff.diff
                .changes
                .iter()
                .flat_map(|run| mark_moved_blocks(run, &lengths.next().expect("one entry per run")))
                .collect()
        })
        .collect()
}

/// Find the longest opposite-side match beginning at each changed line.
fn matching_block_lengths(runs: &[&ChangeRun]) -> Vec<Vec<usize>> {
    let mut added: HashMap<&str, Vec<(usize, usize)>> = HashMap::new();
    let mut lengths = Vec::with_capacity(runs.len());
    for (run_index, run) in runs.iter().enumerate() {
        lengths.push(vec![0; run.lines.len()]);
        if run.kind == ChangeKind::Added {
            for (line_index, line) in run.lines.iter().enumerate() {
                added
                    .entry(moved_line_key(line))
                    .or_default()
                    .push((run_index, line_index));
            }
        }
    }

    for (removed_index, removed) in runs.iter().enumerate() {
        if removed.kind != ChangeKind::Removed {
            continue;
        }
        for (before, line) in removed.lines.iter().enumerate() {
            let Some(candidates) = added.get(moved_line_key(line)) else {
                continue;
            };
            for &(added_index, after) in candidates {
                let added = &runs[added_index].lines;
                if before > 0
                    && after > 0
                    && moved_line_key(&removed.lines[before - 1])
                        == moved_line_key(&added[after - 1])
                {
                    // This pair is inside a match already examined
                    // from its start. Avoid extending it repeatedly.
                    continue;
                }
                let length = removed.lines[before..]
                    .iter()
                    .zip(&added[after..])
                    .take_while(|(old, new)| moved_line_key(old) == moved_line_key(new))
                    .count();
                for offset in 0..length {
                    let remaining = length - offset;
                    lengths[removed_index][before + offset] =
                        lengths[removed_index][before + offset].max(remaining);
                    lengths[added_index][after + offset] =
                        lengths[added_index][after + offset].max(remaining);
                }
            }
        }
    }
    lengths
}

fn moved_line_key(line: &str) -> &str {
    // Git normalizes the final LF for move matching, including an
    // unterminated last line. Preserve all other bytes, including CR.
    line.strip_suffix('\n').unwrap_or(line)
}

fn mark_moved_blocks(run: &ChangeRun, lengths: &[usize]) -> Vec<bool> {
    // Git's blocks mode requires 20 alphanumeric characters. Prefix
    // sums also keep retries within long blank runs inexpensive.
    const MIN_ALNUM_COUNT: usize = 20;

    let mut alnum = Vec::with_capacity(run.lines.len() + 1);
    alnum.push(0);
    for line in &run.lines {
        let count = line.bytes().filter(u8::is_ascii_alphanumeric).count();
        alnum.push(alnum.last().copied().unwrap() + count);
    }

    let mut moved = vec![false; run.lines.len()];
    let mut start = 0;
    while start < run.lines.len() {
        let end = start + lengths[start];
        if alnum[end] - alnum[start] >= MIN_ALNUM_COUNT {
            moved[start..end].fill(true);
            start = end;
        } else {
            // A rejected block may contain the start of a longer one.
            start += 1;
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;
    use ChangeKind::{Added, Removed};

    #[test]
    fn moved_blocks_require_twenty_ascii_alphanumeric_characters() {
        for (text, expected) in [
            ("1234567890\n123456789\n", vec![false; 2]),
            ("1234567890\n1234567890\n", vec![true; 2]),
            ("12345678901234567890\n", vec![true]),
            ("éééééééééééééééééééé\n", vec![false]),
        ] {
            assert_eq!(
                classify(&[(Removed, text), (Added, text)]),
                [expected.clone(), expected],
                "{text:?}"
            );
        }
    }

    #[test]
    fn moved_blocks_include_surrounding_blank_lines_and_punctuation() {
        let text = "\n{\nabcdefghij\nklmnopqrst\n}\n\n";
        assert_eq!(
            classify(&[(Removed, text), (Added, text)]),
            [vec![true; 6], vec![true; 6]]
        );
        assert_eq!(
            classify(&[(Removed, "\n{\n}\n"), (Added, "\n{\n}\n")]),
            [vec![false; 3], vec![false; 3]]
        );
    }

    #[test]
    fn moved_blocks_match_repeated_occurrences_without_consuming_partners() {
        let text = "abcdefghij\nklmnopqrst\n";
        assert_eq!(
            classify(&[
                (Removed, text),
                (Added, text),
                (Added, text),
                (Removed, text),
            ]),
            vec![vec![true; 2]; 4]
        );
    }

    #[test]
    fn moved_blocks_greedily_accept_the_longest_match_on_each_side() {
        assert_eq!(
            classify(&[
                (Removed, "abcdefghijklmnopqrst\nx\n1234567890123456789\n"),
                (Added, "abcdefghijklmnopqrst\nx\n"),
                (Added, "x\n1234567890123456789\n"),
            ]),
            [vec![true, true, false], vec![true; 2], vec![true; 2]]
        );
    }

    #[test]
    fn moved_blocks_retry_inside_a_rejected_short_match() {
        assert_eq!(
            classify(&[
                (Removed, "a\nbbbbbbbbb\nccccccccccc\n"),
                (Added, "a\nbbbbbbbbb\n"),
                (Added, "bbbbbbbbb\nccccccccccc\n"),
            ]),
            [vec![false, true, true], vec![false; 2], vec![true; 2]]
        );
    }

    #[test]
    fn moved_blocks_do_not_join_separate_change_runs() {
        assert_eq!(
            classify(&[
                (Removed, "abcdefghij\n"),
                (Removed, "klmnopqrst\n"),
                (Added, "abcdefghij\nklmnopqrst\n"),
            ]),
            [vec![false], vec![false], vec![false; 2]]
        );
    }

    #[test]
    fn moved_blocks_require_an_opposite_side_match() {
        let text = "abcdefghijklmnopqrst\n";
        assert_eq!(
            classify(&[(Added, text), (Added, text)]),
            vec![vec![false]; 2]
        );
        assert_eq!(
            classify(&[(Removed, text), (Removed, text)]),
            vec![vec![false]; 2]
        );
    }

    #[test]
    fn moved_blocks_compare_whitespace_and_line_endings_exactly() {
        for after in [
            "abcdefghijklmnopqrst\r\n",
            "abcdefghijklmnopqrst \n",
            " abcdefghijklmnopqrst\n",
            "abcdefghij\tklmnopqrst\n",
        ] {
            assert_eq!(
                classify(&[(Removed, "abcdefghijklmnopqrst\n"), (Added, after)]),
                vec![vec![false]; 2],
                "{after:?}"
            );
        }
    }

    #[test]
    fn moved_blocks_ignore_the_final_newline_like_git() {
        assert_eq!(
            classify(&[
                (Removed, "abcdefghijklmnopqrst\n"),
                (Added, "abcdefghijklmnopqrst"),
            ]),
            [vec![true], vec![true]]
        );
    }

    #[test]
    fn moved_blocks_classification_is_symmetric() {
        let before = "unique\nabcdefghij\nklmnopqrst\n";
        let after = "abcdefghij\nklmnopqrst\nother\n";
        let forward = classify(&[(Removed, before), (Added, after)]);
        let reversed = classify(&[(Added, before), (Removed, after)]);
        assert_eq!(forward, [vec![false, true, true], vec![true, true, false]]);
        assert_eq!(forward, reversed);
    }

    #[test]
    fn moved_blocks_handle_long_repetitive_runs() {
        let blanks = "\n".repeat(512);
        assert_eq!(
            classify(&[(Removed, &blanks), (Added, &blanks)]),
            [vec![false; 512], vec![false; 512]]
        );
        let qualifying = format!("{blanks}abcdefghijklmnopqrst\n");
        assert_eq!(
            classify(&[(Removed, &qualifying), (Added, &qualifying)]),
            [vec![true; 513], vec![true; 513]]
        );
    }

    fn classify(runs: &[(ChangeKind, &str)]) -> Vec<Vec<bool>> {
        let diffs = runs
            .iter()
            .enumerate()
            .map(|(index, &(kind, text))| Diff {
                file: index.to_string(),
                diff: FileDiff {
                    text: String::new(),
                    changes: vec![ChangeRun {
                        kind,
                        lines: text.split_inclusive('\n').map(str::to_owned).collect(),
                    }],
                },
            })
            .collect::<Vec<_>>();
        detect_moved_lines(&diffs)
    }
}
