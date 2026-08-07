use std::borrow::Cow;
use std::ffi::OsStr;
use std::process;

use deezconfigs::ui;

use super::common::{ensure_root_exists, get_config_root_from_config};

/// Run command inside config root.
pub fn run(run_args: &[impl AsRef<OsStr>], verbose: bool) -> Result<(), i32> {
    let Some(root) = get_config_root_from_config() else {
        eprintln!(
            "{error}: The 'DEEZ_ROOT' environment variable is not set.",
            error = ui::Color::error("error")
        );
        return Err(1);
    };
    ensure_root_exists(&root)?;

    // Bare `deez run`, not followed by a command.
    let Some((command, args)) = run_args.split_first() else {
        eprintln!("{error}: Run deez what?", error = ui::Color::error("error"));
        return Err(2);
    };

    if verbose {
        println!("root: {}", root.display());
    }

    let status = process::Command::new(command)
        .current_dir(root)
        .args(args)
        .status();

    match status {
        Ok(status) => match status.code() {
            Some(0) => Ok(()),
            Some(code) => Err(code),
            None => Err(1),
        },
        Err(err) => {
            let command = command.as_ref().to_string_lossy(); // Simplify comparison.
            eprintln!(
                "{fatal}: Could not run '{}': {err}",
                // `deez run nuts` -> `fatal: Command '🥜' not found.`
                if command == "nuts" {
                    Cow::from("🥜")
                } else {
                    command
                },
                fatal = ui::Color::error("fatal")
            );
            Err(1)
        }
    }
}
