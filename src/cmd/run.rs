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

use std::process;

use deezconfigs::ui;

use super::common::{ensure_root_exists, get_config_root_from_config};

/// Run command inside config root.
#[allow(clippy::too_many_lines)] // More a procedure than a function.
pub fn run(run_args: &[String], verbose: bool) -> Result<(), i32> {
    let Some(root) = get_config_root_from_config() else {
        eprintln!(
            "{error}: The 'DEEZ_ROOT' environment variable is not set.",
            error = ui::Color::error("error")
        );
        return Err(1);
    };
    ensure_root_exists(&root)?;

    let Some((command, args)) = run_args.split_first() else {
        eprintln!("{error}: Run deez what?", error = ui::Color::error("error"));
        return Err(2);
    };

    if verbose {
        println!("root: {}", root.display());
    }

    let res = process::Command::new(command)
        .current_dir(root)
        .args(args)
        .status();

    if let Ok(status) = res {
        // Not an error, but we want to forward the exit code.
        Err(status.code().unwrap_or(1))
    } else {
        eprintln!(
            "{fatal}: Command '{}' not found.",
            if command == "nuts" { "🥜" } else { command },
            fatal = ui::Color::error("fatal")
        );
        Err(1)
    }
}
