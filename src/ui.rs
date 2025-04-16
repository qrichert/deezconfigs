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
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::sync::LazyLock;

/// `true` if `NO_COLOR` is set and is non-empty.
#[cfg(not(tarpaulin_include))]
#[allow(unreachable_code)]
pub static NO_COLOR: LazyLock<bool> = LazyLock::new(|| {
    #[cfg(test)]
    {
        return false;
    }
    // Contrary to `env::var()`, `env::var_os()` does not require the
    // value to be valid Unicode.
    env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
});

pub const GREEN: &str = "\x1b[0;92m";
pub const YELLOW: &str = "\x1b[0;93m";
pub const RED: &str = "\x1b[0;91m";
pub const BLUE: &str = "\x1b[0;94m";
pub const ATTENUATE: &str = "\x1b[0;90m";
pub const TITLE: &str = "\x1b[1;4m";
pub const RESET: &str = "\x1b[0m";

pub struct Color;

impl Color {
    #[must_use]
    pub fn in_sync(string: &str) -> Cow<str> {
        Self::color(GREEN, string)
    }

    #[must_use]
    pub fn modified(string: &str) -> Cow<str> {
        Self::color(YELLOW, string)
    }

    #[must_use]
    pub fn missing(string: &str) -> Cow<str> {
        Self::color(RED, string)
    }

    #[must_use]
    pub fn symlink(string: &str) -> Cow<str> {
        Self::color(BLUE, string)
    }

    #[must_use]
    pub fn attenuate(string: &str) -> Cow<str> {
        Self::color(ATTENUATE, string)
    }

    #[must_use]
    pub fn title(string: &str) -> Cow<str> {
        Self::color(TITLE, string)
    }

    #[must_use]
    pub fn none(string: &str) -> Cow<str> {
        Cow::Borrowed(string)
    }

    /// Color string of text.
    ///
    /// The string gets colored in a standalone way, meaning  the reset
    /// code is included, so anything appended to the end of the string
    /// will not be colored.
    ///
    /// This function takes `NO_COLOR` into account. In no-color mode,
    /// the returned string will be equal to the input string, no color
    /// gets added.
    #[must_use]
    fn color<'a>(color: &str, string: &'a str) -> Cow<'a, str> {
        if *NO_COLOR {
            #[cfg(not(tarpaulin_include))] // Unreachable in tests.
            return Cow::Borrowed(string);
        }
        Cow::Owned(format!("{color}{string}{RESET}"))
    }
}

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
    print_files_summary(root, nb_files_written, nb_errors);
    print_hooks_summary(nb_hooks_ran);
}

pub fn print_files_summary(root: &Path, nb_files_written: usize, nb_errors: usize) {
    if nb_files_written + nb_errors == 0 {
        println!("No config files found in '{}'.", root.display());
    }

    let mut stdout = io::stdout().lock();

    // TODO: Adapt the word `Wrote` to the action.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_in_sync_is_green() {
        assert_eq!(
            Color::in_sync("this is in sync"),
            "\x1b[0;92mthis is in sync\x1b[0m"
        );
    }

    #[test]
    fn color_modified_is_yellow() {
        assert_eq!(
            Color::modified("this is marked as modified"),
            "\x1b[0;93mthis is marked as modified\x1b[0m"
        );
    }

    #[test]
    fn color_missing_is_red() {
        assert_eq!(
            Color::missing("this is marked as missing"),
            "\x1b[0;91mthis is marked as missing\x1b[0m"
        );
    }

    #[test]
    fn color_symlink_is_blue() {
        assert_eq!(
            Color::missing("this is a symlink"),
            "\x1b[0;91mthis is a symlink\x1b[0m"
        );
    }

    #[test]
    fn color_attenuate_is_grey() {
        assert_eq!(
            Color::attenuate("this is attenuated"),
            "\x1b[0;90mthis is attenuated\x1b[0m"
        );
    }

    #[test]
    fn color_title_is_bold_underlined() {
        assert_eq!(
            Color::title("this is bold, and underlined"),
            "\x1b[1;4mthis is bold, and underlined\x1b[0m"
        );
    }

    #[test]
    fn color_none_has_no_effect() {
        assert_eq!(Color::none("same as input"), "same as input");
    }
}
