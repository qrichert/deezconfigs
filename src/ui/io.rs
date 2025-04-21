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

use std::fmt;
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Action {
    Sync,
    RSync,
    Link,
    Clean,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sync | Self::RSync => write!(f, "Synced"),
            Self::Link => write!(f, "Linked"),
            Self::Clean => write!(f, "Removed"),
        }
    }
}

pub fn print_files(files: &[String]) {
    let mut stdout = io::stdout().lock();
    for file in files {
        _ = writeln!(stdout, "{file}");
    }
}

pub fn print_summary(
    action: Action,
    root: &Path,
    nb_files: usize,
    nb_errors: usize,
    nb_hooks_ran: usize,
) {
    print_files_summary(action, root, nb_files, nb_errors);
    print_hooks_summary(nb_hooks_ran);
}

pub fn print_files_summary(action: Action, root: &Path, nb_files: usize, nb_errors: usize) {
    if nb_files + nb_errors == 0 {
        println!("No config files found in '{}'.", root.display());
    }

    let mut stdout = io::stdout().lock();

    _ = write!(
        stdout,
        "{action} {nb_files} file{}",
        if nb_files == 1 { "" } else { "s" }
    );
    if nb_errors > 0 {
        _ = write!(
            stdout,
            ", {nb_errors} error{}",
            if nb_errors == 1 { "" } else { "s" }
        );
    }
    _ = writeln!(stdout, ".");
}

pub fn print_hooks_summary(nb_hooks_ran: usize) {
    if nb_hooks_ran == 0 {
        return;
    }
    println!(
        "Ran {nb_hooks_ran} hook{}.",
        if nb_hooks_ran == 1 { "" } else { "s" }
    );
}
