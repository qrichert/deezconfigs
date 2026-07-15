// deezconfigs — Manage deez config files.
// Copyright (C) 2026  Quentin Richert
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

/// Generate shared hook tests for a command.
macro_rules! hook_tests {
    ($cmd:ident) => {
        mod $cmd {
            use $crate::utils;
            use $crate::utils::conf::{self, CONFIGS, HOME};
            use $crate::utils::run::run;

            #[test]
            fn hooks_are_executed() {
                conf::init();
                let cmd = stringify!($cmd);

                // (Add 'OK's to differentiate from verbose output.)
                conf::create_executable_file_in_configs(
                    &format!("pre-{cmd}"),
                    Some(&format!("echo 'pre-{cmd} OK'")),
                );
                conf::create_executable_file_in_configs(
                    &format!("pre-{cmd}.sh"),
                    Some(&format!("echo 'pre-{cmd}.sh OK'")),
                );
                conf::create_executable_file_in_configs(
                    &format!("post-{cmd}.sh"),
                    Some(&format!("echo 'post-{cmd}.sh OK'")),
                );

                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(output.stdout.contains(&format!("pre-{cmd} OK\n")));
                assert!(output.stdout.contains(&format!("pre-{cmd}.sh OK\n")));
                assert!(output.stdout.contains(&format!("post-{cmd}.sh OK\n")));

                assert!(output.stdout.contains("Ran 3 hooks."));
            }

            #[test]
            fn hooks_are_executed_in_configs_dir() {
                conf::init();
                let cmd = stringify!($cmd);

                conf::create_executable_file_in_configs(
                    &format!("post-{cmd}.sh"),
                    Some(&format!(r#"echo "post-{cmd}.sh:$(pwd)""#)),
                );

                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(
                    output
                        .stdout
                        .contains(&format!("post-{cmd}.sh:{CONFIGS}\n"))
                );
            }

            #[test]
            fn hooks_are_executed_in_order_of_file_name() {
                conf::init();
                let cmd = stringify!($cmd);

                conf::create_executable_file_in_configs(&format!("post-{cmd}.sh"), None);
                conf::create_executable_file_in_configs(&format!("post-{cmd}.py"), None);
                conf::create_executable_file_in_configs(&format!("post-{cmd}.001.py"), None);
                conf::create_executable_file_in_configs(&format!("post-{cmd}.002.sh"), None);

                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(output.stdout.contains(&format!(
                    "hook: post-{cmd}.001.py\n\
                     hook: post-{cmd}.002.sh\n\
                     hook: post-{cmd}.py\n\
                     hook: post-{cmd}.sh\n"
                )));
            }

            #[test]
            fn hooks_ignore_other_commands_hooks() {
                conf::init();
                let cmd = stringify!($cmd);

                utils::create_all_command_hooks(None);

                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                utils::assert_only_own_hooks_ran(&output, cmd);
            }

            #[test]
            fn hooks_expose_root() {
                conf::init();
                let cmd = stringify!($cmd);

                conf::create_executable_file_in_configs(
                    &format!("pre-{cmd}.sh"),
                    Some(r"echo root=$DEEZ_ROOT"),
                );

                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(output.stdout.contains(&format!("\nroot={CONFIGS}\n")));
            }

            #[test]
            fn hooks_expose_home() {
                conf::init();
                let cmd = stringify!($cmd);

                conf::create_executable_file_in_configs(
                    &format!("pre-{cmd}.sh"),
                    Some(r"echo home=$DEEZ_HOME"),
                );

                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(output.stdout.contains(&format!("\nhome={HOME}\n")));
            }

            #[test]
            fn hooks_expose_verbose_mode() {
                conf::init();
                let cmd = stringify!($cmd);

                conf::create_executable_file_in_configs(
                    &format!("pre-{cmd}.sh"),
                    Some(r#"[ -n "$DEEZ_VERBOSE" ] && echo verbose=true || echo verbose=false"#),
                );

                // Normal run.
                let output = run(&[cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(output.stdout.contains("verbose=false"));

                // Verbose run.
                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(output.stdout.contains("verbose=true"));
            }

            #[test]
            fn hooks_expose_os() {
                conf::init();
                let cmd = stringify!($cmd);

                conf::create_executable_file_in_configs(
                    &format!("pre-{cmd}.sh"),
                    Some(r"echo os=$DEEZ_OS"),
                );

                let output = run(&["--verbose", cmd, &conf::root()]);
                dbg!(&output.stdout);
                dbg!(&output.stderr);

                assert_eq!(output.exit_code, 0);

                assert!(
                    output
                        .stdout
                        .contains(&format!("\nos={}\n", std::env::consts::OS))
                );
            }
        }
    };
}

pub(crate) use hook_tests;
