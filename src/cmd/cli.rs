#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Sync,
    RSync,
    Link,
    Status,
    Diff,
    Clean,
    Run,
    Nuts,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Args {
    pub command: Option<Command>,
    pub pull_before_command: bool,
    pub reversed_diff: bool,
    pub incoming_diff: bool,
    #[allow(clippy::struct_field_names)]
    pub run_args: Vec<String>,
    pub root: Option<String>,
    pub pathspecs: Vec<String>,
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
                "-r" | "--reversed" if is_diff => args.reversed_diff = !args.reversed_diff,
                "-i" | "--incoming" if is_diff => args.incoming_diff = !args.incoming_diff,
                "clean" | "c" if !some_command => args.command = Some(Command::Clean),
                "run" | "r" if !some_command => {
                    args.command = Some(Command::Run);
                    args.run_args
                        .extend(cli_args.by_ref().map(|arg| arg.to_string()));
                }
                "nuts" if !some_command => args.command = Some(Command::Nuts),
                "-h" => args.short_help = true,
                "--help" => args.long_help = true,
                "-V" | "--version" => args.version = true,
                "-v" | "--verbose" => args.verbose = true,
                "-p" | "--pull" => args.pull_before_command = true,
                "--" if some_command => {
                    // Everything after `--` is a pathspec (git-style).
                    // Root is positional and must come _before_ `--`.
                    args.pathspecs
                        .extend(cli_args.by_ref().map(|arg| arg.to_string()));
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
    fn command_sync_pull() {
        let args = Args::build_from_args(["sync", "--pull"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.pull_before_command);
    }

    #[test]
    fn option_pull_is_global_before_command() {
        // `--pull` is a global flag; it works in any position, even
        // before the command.
        let args = Args::build_from_args(["--pull", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.pull_before_command);
    }

    #[test]
    fn option_pull_shortcut() {
        let args = Args::build_from_args(["-p", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.pull_before_command);
    }

    #[test]
    fn option_pull_works_with_any_command() {
        // No longer sync-gated.
        let args = Args::build_from_args(["rsync", "--pull"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::RSync));
        assert!(args.pull_before_command);
    }

    #[test]
    fn option_pull_after_run_is_a_run_argument() {
        // `run` drains all trailing arguments, so `--pull` goes to the
        // child process; it is _not_ consumed as the global flag.
        let args = Args::build_from_args(["run", "--pull"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Run));
        assert!(!args.pull_before_command);
        assert_eq!(args.run_args, ["--pull"]);
    }

    #[test]
    fn option_pull_before_run_is_set_but_ignored_by_dispatch() {
        // Parsed as the global flag (before `run` drains the rest), but
        // `run` doesn't take `pull_before_command`, so dispatch ignores it.
        let args = Args::build_from_args(["--pull", "run"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Run));
        assert!(args.pull_before_command);
        assert!(args.run_args.is_empty());
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
    fn command_diff_reversed() {
        let args = Args::build_from_args(["diff", "--reversed"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
        assert!(args.reversed_diff);
    }

    #[test]
    fn command_reversed_without_diff_is_noop() {
        let args = Args::build_from_args(["status", "--reversed"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Status));
        assert!(!args.reversed_diff);
    }

    #[test]
    fn command_diff_reversed_multiple_cancel_each_other() {
        let args = Args::build_from_args(["diff", "--reversed", "--reversed"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
        assert!(!args.reversed_diff);
    }

    #[test]
    fn command_diff_incoming() {
        let args = Args::build_from_args(["diff", "--incoming"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
        assert!(args.incoming_diff);
    }

    #[test]
    fn command_diff_incoming_shortcut() {
        let args = Args::build_from_args(["diff", "-i"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
        assert!(args.incoming_diff);
    }

    #[test]
    fn command_incoming_without_diff_is_noop() {
        let args = Args::build_from_args(["status", "--incoming"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Status));
        assert!(!args.incoming_diff);
    }

    #[test]
    fn command_diff_incoming_multiple_cancel_each_other() {
        let args = Args::build_from_args(["diff", "--incoming", "--incoming"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
        assert!(!args.incoming_diff);
    }

    #[test]
    fn command_diff_incoming_and_reversed() {
        let args = Args::build_from_args(["diff", "-i", "-r"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Diff));
        assert!(args.incoming_diff);
        assert!(args.reversed_diff);
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
    fn command_run_regular() {
        let args = Args::build_from_args(["run"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Run));
    }

    #[test]
    fn command_run_shortcut() {
        let args = Args::build_from_args(["r"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Run));
    }

    #[test]
    fn second_command_does_not_override_run() {
        let args = Args::build_from_args(["run", "sync"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Run));
    }

    #[test]
    fn command_run_drains_all_remaining_arguments() {
        let args = Args::build_from_args(["run", "git", "pull"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Run));
        assert_eq!(args.run_args, ["git", "pull"]);
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

    #[test]
    fn root_before_double_dash_coexists_with_pathspecs() {
        // Root is positional (before `--`); pathspecs follow.
        let args =
            Args::build_from_args(["sync", "~/other-root", "--", "~/configs"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_some_and(|r| r == "~/other-root"));
        assert_eq!(args.pathspecs, ["~/configs"]);
    }

    #[test]
    fn double_dash_collects_a_pathspec() {
        let args = Args::build_from_args(["sync", "--", "~/configs"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_none());
        assert_eq!(args.pathspecs, ["~/configs"]);
    }

    #[test]
    fn double_dash_not_followed_by_anything_is_noop() {
        let args = Args::build_from_args(["sync", "--"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_none());
        assert!(args.pathspecs.is_empty());
    }

    #[test]
    fn double_dash_drains_everything_after_it_as_pathspecs() {
        // Flag-like tokens after `--` are pathspecs, not options.
        let args = Args::build_from_args(["sync", "--", "--verbose"].iter()).unwrap();
        assert!(args.command.is_some_and(|c| c == Command::Sync));
        assert!(args.root.is_none());
        assert!(!args.verbose);
        assert_eq!(args.pathspecs, ["--verbose"]);
    }

    #[test]
    fn double_dash_drains_multiple_pathspecs() {
        let args = Args::build_from_args(["sync", "--", "a", "b", "c"].iter()).unwrap();
        assert_eq!(args.pathspecs, ["a", "b", "c"]);
    }

    #[test]
    fn double_dash_collects_negation_tokens_verbatim() {
        // The parser only collects strings; `pathspec` validates them.
        let args = Args::build_from_args(["diff", "--", ":!foo", ":^bar"].iter()).unwrap();
        assert_eq!(args.pathspecs, [":!foo", ":^bar"]);
    }

    #[test]
    fn double_dash_not_preceded_by_command_is_error() {
        let err = Args::build_from_args(["--", "~/configs"].iter()).unwrap_err();
        assert!(err.contains("'--'"));
    }

    #[test]
    fn pathspecs_default_empty() {
        let args = Args::build_from_args(["sync"].iter()).unwrap();
        assert!(args.pathspecs.is_empty());
    }
}
