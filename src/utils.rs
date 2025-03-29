use std::io;

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

use std::io::Write;

/// Prompt the user to confirm an action (custom prompt).
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
