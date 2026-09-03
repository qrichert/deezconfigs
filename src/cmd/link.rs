use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use deezconfigs::pathspec::PathSpec;
use deezconfigs::{ui, walk};

use super::common::{
    get_home_directory, get_hooks_for_command, resolve_and_pull_config_root, resolve_config_root,
    run_hooks,
};

/// Link config from root into home.
///
/// 1. Collect all files in `configs`.
/// 2. Create matching symlinks to the files in `$HOME`.
pub fn link(
    root: Option<&String>,
    verbose: bool,
    pull_before_command: bool,
    pathspec: &PathSpec,
) -> Result<(), i32> {
    let root = if pull_before_command {
        resolve_and_pull_config_root(root)?
    } else {
        resolve_config_root(root, true)?
    };
    let home = get_home_directory()?;
    let hooks = get_hooks_for_command(&root, &home, verbose)?;

    let mut nb_hooks_ran = 0;

    nb_hooks_ran += run_hooks(|| hooks.pre_link())?;

    // There will be high contention, but it likely won't matter much
    // given there are rarely _that_ many config files (and the syscalls
    // we issue are a bigger bottleneck anyway).
    let files = Arc::new(Mutex::new(Vec::with_capacity(20)));
    let nb_files_linked = AtomicUsize::new(0);
    let nb_errors = AtomicUsize::new(0);

    walk::find_files_recursively(&root, pathspec, |p| {
        debug_assert!(!p.is_dir());

        let source = root.join(p);
        let destination = home.join(p);

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
                "{error}: Could not link '{}' to home: {err}",
                p.display(),
                error = ui::Color::error("error"),
            );
            return;
        }

        // If destination exists, remove it.
        if destination.is_file() || destination.is_symlink() {
            // TODO: We put `is_symlink()` to handle the case when the
            //  link is broken (and so `is_file()` presumably wouldn't
            //  match?). Test it out.
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

        #[cfg(unix)]
        let res = std::os::unix::fs::symlink(&source, &destination);
        #[cfg(windows)]
        let res = std::os::windows::fs::symlink_file(&source, &destination);

        if let Err(err) = res {
            nb_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "{error}: Could not create link to '{}': {err}",
                source.display(),
                error = ui::Color::error("error"),
            );
            return;
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

        nb_files_linked.fetch_add(1, Ordering::Relaxed);
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

    nb_hooks_ran += run_hooks(|| hooks.post_link())?;

    let nb_files_linked = nb_files_linked.into_inner();
    let nb_errors = nb_errors.into_inner();

    ui::print_summary(
        ui::Action::Link,
        &root,
        nb_files_linked,
        None,
        nb_errors,
        nb_hooks_ran,
    );

    if nb_errors > 0 { Err(1) } else { Ok(()) }
}
