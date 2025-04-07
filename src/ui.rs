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

use std::io::{self, Write};
use std::path::Path;

/// Prompt the user to confirm an action (custom prompt).
#[must_use]
pub fn ask_confirmation_with_prompt(prompt: &str) -> bool {
    print!("{prompt} (y/N) ");
    _ = io::stdout().flush();

    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        eprintln!("Error reading user input.");
        return false;
    }

    matches!(answer.to_ascii_lowercase().trim(), "y" | "yes")
}

pub fn print_summary(root: &Path, nb_files_written: usize, nb_errors: usize, nb_hooks_ran: usize) {
    let mut stdout = io::stdout().lock();

    // Config files.
    if nb_files_written + nb_errors > 0 {
        _ = write!(
            stdout,
            "Wrote {nb_files_written} file{}",
            if nb_files_written == 1 { "" } else { "s" }
        );
        if nb_errors > 0 {
            _ = write!(
                stdout,
                ", {nb_errors} error{}",
                if nb_errors == 1 { "" } else { "s" }
            );
        }
        _ = writeln!(stdout, ".");
    } else {
        _ = writeln!(stdout, "No config files found in '{}'.", root.display());
    }

    // Hooks.
    if nb_hooks_ran > 0 {
        _ = writeln!(
            stdout,
            "Ran {nb_hooks_ran} hook{}.",
            if nb_hooks_ran == 1 { "" } else { "s" }
        );
    }
}
