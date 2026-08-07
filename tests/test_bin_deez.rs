mod utils;

use utils::run::run;

#[test]
fn help() {
    let output = run(&["--help"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
    assert!(output.stdout.contains("-V, --version"));
    assert!(output.stdout.contains("-v, --verbose"));
    assert!(output.stdout.contains("sync [<root>|<git>]"));
    assert!(output.stdout.contains("rsync [<root>]"));
    assert!(output.stdout.contains("link [<root>]"));
    assert!(output.stdout.contains("-r, --reversed"));
    assert!(output.stdout.contains("-i, --incoming"));
    // Pin the direction of `diff`, not just the flag's existence. Both
    // comments exist either way; only their pairing with the example
    // command tells the two directions apart. The trailing newline
    // matters: `$ deez diff` is a prefix of `$ deez diff -r`.
    assert!(
        output
            .stdout
            .contains("# Compare the home (old) to the config root (new).\n      $ deez diff\n")
    );
    assert!(
        output
            .stdout
            .contains("# Compare the config root (old) to the home (new).\n      $ deez diff -r\n")
    );
    assert!(output.stdout.contains(
        "\
  Available hooks:

      pre-sync    post-sync
      pre-rsync   post-rsync
      pre-link    post-link
      pre-status  post-status
      pre-diff    post-diff
      pre-clean   post-clean"
    ));
}

#[test]
fn no_args_shows_help() {
    let output = run(&[]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("-h, --help"));
}

#[test]
fn version() {
    let output = run(&["--version"]);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));

    let output = run(&["-V"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn bad_argument() {
    let output = run(&["--bad-argument"]);
    dbg!(&output.stdout);
    dbg!(&output.stderr);

    assert_eq!(output.exit_code, 2);
    assert!(output.stderr.contains("'--bad-argument'"));
}
