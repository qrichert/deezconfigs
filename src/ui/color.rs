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
pub const BOLD_PURPLE: &str = "\x1b[1;95m";
pub const CYAN: &str = "\x1b[0;96m";
pub const RESET: &str = "\x1b[0m";

pub const HIGHLIGHT: &str = GREEN;
pub const ATTENUATE: &str = "\x1b[0;90m";
pub const UNDERLINE: &str = "\x1b[4m";

pub struct Color;

impl Color {
    // Errors.

    #[must_use]
    pub fn error(string: &str) -> Cow<str> {
        Self::color(RED, string)
    }

    #[must_use]
    pub fn warning(string: &str) -> Cow<str> {
        Self::color(YELLOW, string)
    }

    // Status.

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

    // Diff.

    #[must_use]
    pub fn file_name(string: &str) -> Cow<str> {
        Self::color(BOLD_PURPLE, string)
    }

    #[must_use]
    pub fn line_range(string: &str) -> Cow<str> {
        Self::color(CYAN, string)
    }

    #[must_use]
    pub fn added(string: &str) -> Cow<str> {
        Self::color(GREEN, string)
    }

    #[must_use]
    pub fn removed(string: &str) -> Cow<str> {
        Self::color(RED, string)
    }

    // Generic.

    /// Return string without adding color.
    ///
    /// The purpose of this function is uniformity.
    ///
    /// ```
    /// # use std::borrow::Cow;
    /// # use deezconfigs::ui::Color;
    /// # let x = true;
    /// // Very nice:
    /// let color = if x {
    ///     Color::in_sync("...")
    /// } else {
    ///     Color::none("...")
    /// };
    ///
    /// // Not nice:
    /// let color = if x {
    ///     Color::in_sync("...")
    /// } else {
    ///     Cow::Borrowed("...")
    /// };
    /// ```
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

    /// Return input color, or nothing in no-color mode.
    ///
    /// This makes it easy to support no-color mode.
    ///
    /// Wrap color code strings in this function. In regular mode, it
    /// will return the string as-is. But it no-color mode, it will
    /// return an empty string.
    ///
    /// This can be used if you don't want to use the pre-defined
    /// coloring functions. It is lower level, but nicer than manually
    /// checking the [`NO_COLOR`] static variable.
    ///
    /// ```ignore
    /// // In regular colored-mode.
    /// assert_eq(
    ///     Color::maybe_color("\x1b[96m"),
    ///     "\x1b[96m",
    /// );
    ///
    /// // In no-color mode.
    /// assert_eq(
    ///     Color::maybe_color("\x1b[96m"),
    ///     "",
    /// )
    /// ```
    #[must_use]
    pub fn maybe_color(color: &str) -> &str {
        if *NO_COLOR {
            #[cfg(not(tarpaulin_include))] // Unreachable in tests.
            return "";
        }
        color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_error_is_red() {
        assert_eq!(
            Color::error("this is an error"),
            "\x1b[0;91mthis is an error\x1b[0m"
        );
    }

    #[test]
    fn color_warning_is_yellow() {
        assert_eq!(
            Color::warning("this is a warning"),
            "\x1b[0;93mthis is a warning\x1b[0m"
        );
    }

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
    fn color_file_name_is_bold_purple() {
        assert_eq!(
            Color::file_name("this is bold, and purple"),
            "\x1b[1;95mthis is bold, and purple\x1b[0m"
        );
    }

    #[test]
    fn color_line_range_is_cyan() {
        assert_eq!(
            Color::line_range("this is cyan"),
            "\x1b[0;96mthis is cyan\x1b[0m"
        );
    }

    #[test]
    fn color_added_is_green() {
        assert_eq!(
            Color::added("+this is has been added"),
            "\x1b[0;92m+this is has been added\x1b[0m"
        );
    }

    #[test]
    fn color_removed_is_red() {
        assert_eq!(
            Color::removed("+this is has been removed"),
            "\x1b[0;91m+this is has been removed\x1b[0m"
        );
    }

    #[test]
    fn color_none_has_no_effect() {
        assert_eq!(Color::none("same as input"), "same as input");
    }
}
