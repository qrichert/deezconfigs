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

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Sync,
    RSync,
    Link,
    Status,
    Diff,
    Clean,
    Nuts,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Args {
    pub command: Option<Command>,
    pub reversed_diff: bool,
    pub root: Option<String>,
    pub short_help: bool,
    pub long_help: bool,
    pub version: bool,
    pub verbose: bool,
}

impl Args {
    pub fn build_from_args<I>(mut cli_args: I) -> Result<Self, String>
    where
        I: Iterator<Item: AsRef<str> + ToString>,
    {
        let mut args = Self::default();

        while let Some(arg) = cli_args.next() {
            let some_command = args.command.is_some();
            let some_root = args.root.is_some();

            let is_diff = args.command == Some(Command::Diff);

            match arg.as_ref() {
                "sync" | "s" if !some_command => args.command = Some(Command::Sync),
                "rsync" | "rs" if !some_command => args.command = Some(Command::RSync),
                "link" | "l" if !some_command => args.command = Some(Command::Link),
                "status" | "st" if !some_command => args.command = Some(Command::Status),
                "diff" | "df" if !some_command => args.command = Some(Command::Diff),
                "-r" | "--reversed" if is_diff => args.reversed_diff = true,
                "clean" | "c" if !some_command => args.command = Some(Command::Clean),
                "nuts" if !some_command => args.command = Some(Command::Nuts),
                "-h" => args.short_help = true,
                "--help" => args.long_help = true,
                "-V" | "--version" => args.version = true,
                "-v" | "--verbose" => args.verbose = true,
                "--" if some_command && !some_root => {
                    args.root = cli_args.next().map(|root| root.to_string());
                }
                root if some_command && !some_root => args.root = Some(root.to_string()),
                unknown => {
                    return Err(format!("Unknown argument: '{unknown}'"));
                }
            }
        }

        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::iter_on_single_items)]

    use super::*;

    #[test]
    fn command_sync_regular() {
        let args = Args::build_from_args(["sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
    }

    #[test]
    fn command_sync_shortcut() {
        let args = Args::build_from_args(["s"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
    }

    #[test]
    fn second_command_does_not_override_sync() {
        let args = Args::build_from_args(["sync", "rsync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
    }

    #[test]
    fn command_rsync_regular() {
        let args = Args::build_from_args(["rsync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::RSync));
    }

    #[test]
    fn command_rsync_shortcut() {
        let args = Args::build_from_args(["rs"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::RSync));
    }

    #[test]
    fn second_command_does_not_override_rsync() {
        let args = Args::build_from_args(["rsync", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::RSync));
    }

    #[test]
    fn command_link_regular() {
        let args = Args::build_from_args(["link"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Link));
    }

    #[test]
    fn command_link_shortcut() {
        let args = Args::build_from_args(["l"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Link));
    }

    #[test]
    fn second_command_does_not_override_link() {
        let args = Args::build_from_args(["link", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Link));
    }

    #[test]
    fn command_status_regular() {
        let args = Args::build_from_args(["status"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Status));
    }

    #[test]
    fn command_status_shortcut() {
        let args = Args::build_from_args(["st"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Status));
    }

    #[test]
    fn second_command_does_not_override_status() {
        let args = Args::build_from_args(["status", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Status));
    }

    #[test]
    fn command_diff_regular() {
        let args = Args::build_from_args(["diff"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
    }

    #[test]
    fn command_diff_shortcut() {
        let args = Args::build_from_args(["df"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
    }

    #[test]
    fn second_command_does_not_override_diff() {
        let args = Args::build_from_args(["diff", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
    }

    #[test]
    fn command_clean_regular() {
        let args = Args::build_from_args(["clean"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Clean));
    }

    #[test]
    fn command_clean_shortcut() {
        let args = Args::build_from_args(["c"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Clean));
    }

    #[test]
    fn second_command_does_not_override_clean() {
        let args = Args::build_from_args(["clean", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Clean));
    }

    #[test]
    fn command_nuts_regular() {
        let args = Args::build_from_args(["nuts"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Nuts));
    }

    #[test]
    fn second_command_does_not_override_nuts() {
        let args = Args::build_from_args(["nuts", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Nuts));
    }

    #[test]
    fn command_unknown_is_error() {
        let err = Args::build_from_args(["unknown"].iter()).unwrap_err();
        assert!(err.contains("'unknown'"));
    }

    #[test]
    fn option_short_help_regular() {
        let args = Args::build_from_args(["-h"].iter()).unwrap();
        assert!(args.short_help);
        assert!(!args.long_help);
    }

    #[test]
    fn option_long_help_regular() {
        let args = Args::build_from_args(["--help"].iter()).unwrap();
        assert!(!args.short_help);
        assert!(args.long_help);
    }

    #[test]
    fn option_short_version_regular() {
        let args = Args::build_from_args(["-V"].iter()).unwrap();
        assert!(args.version);
    }

    #[test]
    fn option_long_version_regular() {
        let args = Args::build_from_args(["--version"].iter()).unwrap();
        assert!(args.version);
    }

    #[test]
    fn option_short_verbose_regular() {
        let args = Args::build_from_args(["-v"].iter()).unwrap();
        assert!(args.verbose);
    }

    #[test]
    fn option_long_verbose_regular() {
        let args = Args::build_from_args(["--verbose"].iter()).unwrap();
        assert!(args.verbose);
    }

    #[test]
    fn double_dash_regular() {
        let args = Args::build_from_args(["sync", "--", "~/configs"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_some_and(|r| r == "~/configs"));
    }

    #[test]
    fn double_dash_not_followed_by_anything_is_noop() {
        let args = Args::build_from_args(["sync", "--"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_none());
    }

    #[test]
    fn double_dash_correctly_interprets_what_comes_next_as_root() {
        let args = Args::build_from_args(["sync", "--", "--verbose"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_some_and(|r| r == "--verbose"));
        assert!(!args.verbose);
    }

    #[test]
    fn double_dash_not_preceded_by_command_is_error() {
        let err = Args::build_from_args(["--", "~/configs"].iter()).unwrap_err();
        assert!(err.contains("'--'"));
    }

    #[test]
    fn double_dash_with_previous_root_is_error() {
        let err =
            Args::build_from_args(["sync", "~/other-root", "--", "~/configs"].iter()).unwrap_err();
        assert!(err.contains("'--'"));
    }

    #[test]
    fn root_regular() {
        let args = Args::build_from_args(["sync", "~/configs"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_some_and(|r| r == "~/configs"));
    }

    #[test]
    fn root_implicit_is_noop() {
        let args = Args::build_from_args(["sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_none());
    }

    #[test]
    fn root_not_preceded_by_command_is_error() {
        let err = Args::build_from_args(["~/configs"].iter()).unwrap_err();
        assert!(err.contains("'~/configs'"));
    }

    #[test]
    fn root_with_previous_root_is_error() {
        let err = Args::build_from_args(["sync", "~/other-root", "~/configs"].iter()).unwrap_err();
        assert!(err.contains("'~/configs'"));
    }
}
