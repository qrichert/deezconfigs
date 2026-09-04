use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use deezconfigs::pathspec::PathSpec;
use deezconfigs::{ui, utils, walk};

use super::common::{
    get_config_root_from_git, get_home_directory, get_hooks_for_command, is_git_remote_uri,
    resolve_and_pull_config_root, resolve_config_root, run_hooks,
};

/// Sync config from root into home.
///
/// 1. Collect all files in `configs`.
/// 2. Create or replace matching files in `$HOME`.
#[allow(clippy::too_many_lines)] // More a procedure than a function.
pub fn sync(
    root: Option<&String>,
    verbose: bool,
    pull_before_command: bool,
    pathspec: &PathSpec,
) -> Result<(), i32> {
    let root = if pull_before_command {
        resolve_and_pull_config_root(root)?.into()
    } else if is_git_remote_uri(root) {
        get_config_root_from_git(root.expect("not empty, contains a `git:` prefix"), verbose)?
    } else {
        resolve_config_root(root, true)?.into()
    };
    let root: &Path = root.as_ref();
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_sync())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let files = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_files_synced = AtomicUsize::new(0);
    let nb_files_updated = AtomicUsize::new(0);
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(root, pathspec, |p| {
        debug_assert!(!p.is_dir());

        let source = root.join(p);
        let destination = home.join(p);

        let do_source_and_destination_differ =
            verbose && do_source_and_destination_differ(&source, &destination);

        if destination.is_dir() {
            // If destination exists and is a directory, try to `rmdir`
            // it. If it works, the directory was empty anyway. If it
            // doesn't work, the directory is not empty so we abort
            // because it is too risky to remove an entire tree.
            if let Err(err) = fs::remove_dir(&destination) {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not remove exising directory '{}': {err}",
                    destination.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }
        }

        if let Err(err) = fs::create_dir_all(
            destination
                .parent()
                .expect("at the bare minimum, `parent` is `$HOME`"),
        ) {
            nb_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "{error}: Could not copy '{}' to home: {err}",
                p.display(),
                error = ui::Color::error("error"),
            );
            return;
        }

        // If _source_ is a symlink, copy the link, _not_ the contents.
        // We want to _mirror_ what the user has, not interpret what he
        // might have wanted to do.
        //
        // `fs::copy()` follows symlinks. It will create files with the
        // contents of the symlink's target; it will not create a link.
        if source.is_symlink() {
            // If destination exists we must _delete_ it before the
            // copy, because symlinks don't override existing files.
            if destination.is_file() {
                // Matches both files and symlinks.
                if let Err(err) = fs::remove_file(&destination) {
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "{error}: Could not remove exising file '{}': {err}",
                        destination.display(),
                        error = ui::Color::error("error"),
                    );
                    return;
                }
            }

            let target: PathBuf = match fs::read_link(&source) {
                Ok(target) => target,
                Err(err) => {
                    nb_errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "{error}: Could not read symlink '{}': {err}",
                        p.display(),
                        error = ui::Color::error("error"),
                    );
                    return;
                }
            };

            #[cfg(unix)]
            let res = std::os::unix::fs::symlink(&target, &destination);
            #[cfg(windows)]
            let res = std::os::windows::fs::symlink_file(&target, &destination);

            if let Err(err) = res {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not create symlink '{}': {err}",
                    p.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }
        } else {
            // If destination exists and is a symlink, we must _delete_
            // it before the copy, or else it would override the link's
            // target.
            if destination.is_symlink()
                && let Err(err) = fs::remove_file(&destination)
            {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not remove exising symlink '{}': {err}",
                    destination.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }

            if let Err(err) = utils::copy_file(&source, &destination) {
                nb_errors.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "{error}: Could not copy '{}' to home: {err}",
                    p.display(),
                    error = ui::Color::error("error"),
                );
                return;
            }
        }

        if do_source_and_destination_differ {
            nb_files_updated.fetch_add(1, Ordering::Relaxed);
        }

        if verbose {
            let file = p.to_string_lossy().to_string();
            if let Ok(mut files) = files.lock() {
                files.push(file);
                // Release the lock ASAP.
                drop(files);
            } else {
                // It's so unlikely we don't acquire the lock that we
                // just silently fall back to printing directly.
                println!("{}", p.display());
            }
        }

        nb_files_synced.fetch_add(1, Ordering::Relaxed);
    });

    let mut files = Arc::try_unwrap(files)
        .expect("processing is over, we're back to a single thread.")
        .into_inner()
        .unwrap();
    // Do not use `sort_unstable()` because the files are likely
    // _partially_ sorted, in which case stable sort is faster,
    // as per the docs.
    files.sort();

    ui::print_files(&files);

    nb_hooks_ran += run_hooks(|| hooks.post_sync())?;

    let nb_files_synced = nb_files_synced.into_inner();
    let nb_files_updated = nb_files_updated.into_inner();
    let nb_errors = nb_errors.into_inner();

    ui::print_summary(
        ui::Action::Sync,
        root,
        nb_files_synced,
        verbose.then_some(nb_files_updated),
        nb_errors,
        nb_hooks_ran,
    );

    if nb_errors > 0 { Err(1) } else { Ok(()) }
}

fn do_source_and_destination_differ(source: &Path, destination: &Path) -> bool {
    let is_source_symlink = source.is_symlink();
    let is_destination_symlink = destination.is_symlink();

    if is_source_symlink != is_destination_symlink {
        return true;
    }
    if is_source_symlink {
        return match (fs::read_link(source), fs::read_link(destination)) {
            (Ok(source_target), Ok(destination_target)) => source_target != destination_target,
            // If can't compare, assume changed.
            _ => true,
        };
    }
    if !destination.is_file() {
        // Includes missing destination file (will be created).
        return true;
    }

    match utils::are_files_equal(source, destination) {
        Ok(equal) => !equal,
        // If can't compare, assume changed.
        Err(_) => true,
    }
}
