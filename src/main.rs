mod cmd;

use std::env;
use std::process;

use lessify::Pager;

use deezconfigs::ui;

use cmd::cli;

fn main() {
    let args = match cli::Args::build_from_args(env::args().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{fatal}: {err}.", fatal = ui::Color::error("fatal"));
            println!("Try '{bin} -h' for help.", bin = env!("CARGO_BIN_NAME"));
            process::exit(2);
        }
    };

    if args.long_help {
        long_help();
    } else if args.short_help {
        short_help();
    } else if args.version {
        version();
    } else if let Some(command) = args.command {
        let root = args.root.as_ref();
        let verbose = args.verbose;

        if let Err(code) = match command {
            cli::Command::Sync => cmd::sync(root, verbose, args.pull_before_command),
            cli::Command::RSync => cmd::rsync(root, verbose, args.pull_before_command),
            cli::Command::Link => cmd::link(root, verbose, args.pull_before_command),
            cli::Command::Status => cmd::status(root, verbose, args.pull_before_command),
            cli::Command::Diff => {
                if args.incoming_diff {
                    cmd::diff_incoming(root, verbose, args.pull_before_command, args.reversed_diff)
                } else {
                    cmd::diff(root, verbose, args.pull_before_command, args.reversed_diff)
                }
            }
            cli::Command::Clean => cmd::clean(root, verbose, args.pull_before_command),
            cli::Command::Run => cmd::run(&args.run_args, verbose),
            cli::Command::Nuts => {
                println!("Ha! Got 'em!");
                Ok(())
            }
        } {
            process::exit(code);
        }
    } else {
        // No arguments.
        short_help();
    }
}

fn short_help() {
    println!("{}", short_help_message());
    println!(
        "For full help, see `{bin} --help`.",
        bin = env!("CARGO_BIN_NAME")
    );
}

fn short_help_message() -> String {
    format!(
        "\
{description}

Usage: {bin} [<options>] <command> [<args>]

Commands:
  sync [<root>|<git>]    Update home from configs
  rsync [<root>]         Update configs from home
  link [<root>]          Symlink configs to home

  status [<root>|<git>]  List files and their status
  diff [<root>|<git>]    Show what has changed
    -r, --reversed
    -i, --incoming
  clean [<root>|<git>]   Remove all configs from home

  run                    Run command inside the root

Options:
  -p, --pull             Git-pull the root first
  -v, --verbose          Show files being copied

  -h, --help             Show this message and exit
  -V, --version          Show the version and exit
",
        description = env!("CARGO_PKG_DESCRIPTION"),
        bin = env!("CARGO_BIN_NAME"),
    )
}

#[allow(clippy::too_many_lines)]
fn long_help() {
    Pager::page_or_print(&format!(
        "\
{help}
What does {package} do?
  The core of {package} is to replicate the file structure of a given
  directory (i.e., the config root), inside the home. The main purpose
  of this is to keep all the config files in one place, making it easy
  to version them.

  {package} is very unopinionated by default. It tries to do its job
  well (syncing config files), while avoiding doing what other tools do
  better. For instance, there is no automatic versioning, no embedded
  text editor, and no templating. You absolutely {i}can{rt} do all of the
  above, but it's not something that's forced on you. It's {i}your{rt}
  processes, {i}your{rt} tools. All the extensibility power lies in hooks
  (read further below).

Copying vs. Linking:
  {package} supports two configuration models: copying and linking.
  Both models come with different trade-offs. For instance, linking
  ensures files are always up-to-date, but on the flip-side, you can't
  really have machine-specific configuration. On the other hand, copied
  files need to be kept up-to-date manually by `sync`ing or `rsync`ing
  all changes. But, having separate copies makes it easier to keep
  configuration generic in the root, and specific in the home.

The Config Root:
  As mentioned before, the config root is any directory whose structure
  you want to replicate in the home directory.

  {package} will use the root you provide as an argument on the CLI, or
  default to the current working directory.

  You {i}should{rt}, but are not required to, create a `.deez` file in
  the root. This lets {package} know that it is safe to use. If
  {package} doesn't find a `.deez` file, it will ask for confirmation
  before modifying your file system. This is a security feature to
  prevent you from accidentally messing up your home if you run `{bin}`
  from the wrong directory.

  Another advantage of creating a proper root is that it lets you run
  `{bin}` inside sub-directories as well. Just like you can run Git
  commands from anywhere in the repo, {package} is smart enough to
  search for a root in parent directories before warning you that the
  current directory is not a root.

  If you always use the same config root (common case), you can point
  the `DEEZ_ROOT` environment variable to it. In this case, {package}
  will default to it if no root is given on the CLI, and neither the
  current directory, nor its parents are a root. This lets you run
  `{bin}` from anywhere with much less typing.

      {attenuate}# Will status `/home/deez/root` wherever you are.{rt}
      {highlight}${rt} export DEEZ_ROOT=/home/deez/root
      {highlight}${rt} deez status

Home:
  This is the directory where config files are copied or symlinked to.
  On Unix, this is read from the `HOME` environment variable, and on
  Windows from `USERPROFILE`.

  Using a different home is not natively supported by an argument, but
  you can override the environment variable to achieve what you want.

      {highlight}${rt} HOME=/home/other {bin} sync

Sync:
  Syncing in {package} replicates the file structure from the config
  root inside the home directory (minus ignored files).

      {attenuate}# Sync current config root.{rt}
      {highlight}${rt} {bin} sync

      {attenuate}# Sync given config root, verbosely.{rt}
      {highlight}${rt} {bin} --verbose sync ~/configs

      {attenuate}# Sync from remote.{rt}
      {highlight}${rt} {bin} sync https://github.com/qrichert/configs

rSync:
  Reverse-syncing reverses the direction of syncing: it updates your
  config files in the root with the current content from home.

      {attenuate}# 1. Sync your config file to your home.{rt}
      {highlight}${rt} {bin} sync

      {attenuate}# 2. Make some changes.{rt}
      {highlight}${rt} vim ~/.gitconfig

      {attenuate}# 3. rSync the changes back into your root.{rt}
      {highlight}${rt} {bin} rsync

Link:
  Linking is the same as syncing, but it creates symbolic links in the
  home instead of copying files. Linking has no `rsync` equivalent
  because linked files are always up-to-date.

      {attenuate}# Symlink current config root.{rt}
      {highlight}${rt} {bin} link

Status:
  Status prints the list of configuration files with their respective
  state of 'syncness', and also prints your hooks.

  Configuration files can be:

      {in_sync}  In Sync
      {modified}  Modified
      {missing}  Missing

Diff:
  Diffing prints the line-diff between your config root and your home.
  This shows you exactly what has changed and where. There is no merge
  feature, however, as merging is best done by your VCS.

  By default, `diff` uses the home as the {i}before{rt}, and the config root
  as the {i}after{rt}. This assumes you make changes inside the config root,
  and want to see what would change in your home if you `sync`ed
  the updates to it.

      {attenuate}# Compare the home (old) to the config root (new).{rt}
      {highlight}${rt} {bin} diff

  If you make changes in the home directly, however, it is more natural
  to use the config root as the {i}before{rt}, and the home as the {i}after{rt}. In
  other words, you want to see what would change in your root if you
  `rsync`ed the updates back.

  To do this, use the `--reversed` flag:

      {attenuate}# Compare the config root (old) to the home (new).{rt}
      {highlight}${rt} {bin} diff -r

  Note that a config that exists in the root but not in the home is
  reported as missing, not shown as a whole new file, even though
  `sync` would create it.

  Finally, `--incoming` shows what your Git remote has that you don't.
  It is roughly equivalent to running `git fetch` inside the config
  root, followed by `git diff HEAD...<upstream>`:

      {attenuate}# See what a `git pull` would bring in.{rt}
      {highlight}${rt} {bin} diff -i

  Combined with `--reversed`, it shows the opposite: what you have that
  the upstream doesn't.

      {attenuate}# See what you haven't shared yet.{rt}
      {highlight}${rt} {bin} diff -i -r

Clean:
  Cleaning is removing all the files and symlinks from the home.

      {attenuate}# 1. Link your files to your home.{rt}
      {highlight}${rt} {bin} link

      {attenuate}# 2. Now remove all the links you've just created.{rt}
      {highlight}${rt} {bin} clean

Run:
  There is an additional `run` convenience-command which works with the
  `DEEZ_ROOT` environment variable.

  Sometimes, you just want to run a single command in the config root,
  like a `git pull` to get the latest changes. It can be annoying to
  `cd` into the root just for that, and that's where `run` shines:

      {attenuate}# Will run in `/home/deez/root` wherever you are.{rt}
      {highlight}${rt} export DEEZ_ROOT=/home/deez/root
      {highlight}${rt} deez run pwd
      /home/deez/root

      {attenuate}# Run your editor inside the root.{rt}
      {highlight}${rt} deez run $EDITOR

      {attenuate}# Start an interactive shell inside the root.{rt}
      {highlight}${rt} deez run $SHELL

      {attenuate}# A common combination would be:{rt}
      {highlight}${rt} deez run git pull
      {highlight}${rt} deez sync

Shortcuts:
  Each command has a shortcut:

      sync   {u}s{rt}     status  {u}st{rt}
      rsync  {u}rs{rt}    diff    {u}df{rt}
      link   {u}l{rt}     clean   {u}c{rt}
      run    {u}r{rt}

Ignore some files:
  By default, {package} ignores all the hook files (at the root), the
  `.git` directory at the root (if any), all `.ignore` and `.gitignore`
  files, and all `.deez` files, wherever they are (enabling multi-root
  repos).

  You can extend this list by adding entries to your `.ignore` and/or
  `.gitignore` files; they are both respected by {package}.

  If you want to both version a file in Git and have it ignored by
  {package}, you can either add it to a `.gitignore` and force-add it
  with `git add -f`, or you can use a generic `.ignore` file instead.

Git:
  Git is optional, but {package} is designed to integrate nicely with
  it. Beyond respecting `.gitignore` files, {package} can use any Git
  remote as config root with `sync`, `status`, `diff` and `clean`.

  To expand on a previous example:

      {attenuate}# Sync from remote.{rt}
      {highlight}${rt} {bin} sync https://github.com/qrichert/configs

  This will clone the repository to a temporary directory behind the
  scenes, and update your home with its contents. This is useful in
  places where you don't want to maintain a proper clone, and always
  just want to get the latest version.

  {package} considers a Git root any root starting with either `git:`,
  `ssh://`, `git@`, `https://`, or `http://`. `git:` is a special label
  you can use to force a path to be considered a Git root.

  In addition, `gh:` will be replaced with `git@github.com:` (e.g.,
  `gh:qrichert/configs`).

  Furthermore, you can specify a sub-root like this:

      {attenuate}# Sync sub-root.{rt}
      {highlight}${rt} {bin} sync gh:qrichert/configs[sub/directory]

  Instead of assuming the root to be at the repository root, this allows
  using a sub-directory as the root.

  If you're using Git, you can also pull and run a command in one shot
  with the `--pull` flag:

      {attenuate}# Run `git pull` in the config root, then sync.{rt}
      {highlight}${rt} {bin} sync --pull

  This flag only works with local roots; remote roots are always freshly
  cloned.

  And if you only want to {i}see{rt} what a pull would bring in, without
  pulling anything, use `diff --incoming` (`diff` only):

      {attenuate}# Fetch, then show the incoming patch.{rt}
      {highlight}${rt} {bin} diff --incoming

  Like `--pull`, this only works with local roots.

Hooks:
  {package} lets you run hooks before and after commands. Hooks are
  scripts or executables located at the root and whose names match the
  following pattern:

      (pre|post)-<command>[.extension]

  Available hooks:

      pre-sync    post-sync
      pre-rsync   post-rsync
      pre-link    post-link
      pre-status  post-status
      pre-diff    post-diff
      pre-clean   post-clean

  A common example would be...

      post-sync.sh

  ...a shell script that gets run after every `{bin} sync` command.

  You can have multiple hooks for the same action; they will be run in
  name order (`post-sync.001.sh`, then `post-sync.002.sh`, etc.).

  Hooks are executed through `sh`. It is roughly equivalent to:

      {highlight}${rt} cd <root>
      {highlight}${rt} export DEEZ_...  {attenuate}# {bin} environment variables.{rt}
      {highlight}${rt} sh -c \"<root>/<hook>\"

  Note that you'll likely want the scripts to start with a shebang
  (e.g., `#!/usr/bin/env python3`).

  As an example, here are two complementary scripts that respectively
  set and unset Git's email address in the `.gitconfig` file when you
  `sync` and `rsync` it:

      {highlight}${rt} cat post-sync.sh
      #!/usr/bin/env bash
      [[ -n $DEEZ_VERBOSE ]] && echo \"Set global Git email address.\"
      git config --global user.email you@example.com

      {highlight}${rt} cat post-rsync.sh
      #!/usr/bin/env bash
      [[ -n $DEEZ_VERBOSE ]] && echo \"Unset Git email address.\"
      git config --file ./.gitconfig user.email '<>'

  They both make use of the `DEEZ_VERBOSE` environment variable to
  enrich the output of `{bin}` in verbose mode.

  {package} passes a few environment variables to hooks to make your
  life easier:

  - `DEEZ_ROOT` Absolute path to the config root. This is equal to `pwd`
    on Unix systems, since hooks are run in the root.
  - `DEEZ_HOME` Absolute path to the home directory. This is equal to
    `$HOME` on Unix systems.
  - `DEEZ_VERBOSE` Will be `true` if run in verbose mode, otherwise it
    will be unset (hint: use `[[ -n $DEEZ_VERBOSE ]]` to test for
    existence).
  - `DEEZ_OS` Contains the name of the current operating system (e.g.,
    `linux`, `macos`, `windows`, etc.). The name is a re-export of
    Rust's `std::consts::OS`.

Templating:
  There is no built-in templating in {package}, but hooks let you
  implement anything from simple `sed` commands to more advanced
  templating with Jinja2 in Python.

Copy some files, and link others:
  Use multiple roots. You can have multiple roots (sub-directories) in
  one repo. Use `sync` in one, and `link` in the other.

  If you need anything more advanced than that, `{package}` is likely
  not the right tool for you.
",
        help = short_help_message(),
        bin = env!("CARGO_BIN_NAME"),
        package = env!("CARGO_PKG_NAME"),
        highlight = ui::Color::maybe_color(ui::color::HIGHLIGHT),
        attenuate = ui::Color::maybe_color(ui::color::ATTENUATE),
        i = ui::Color::maybe_color(ui::color::ITALIC),
        u = ui::Color::maybe_color(ui::color::UNDERLINE),
        in_sync = ui::Color::in_sync("S"),
        modified = ui::Color::modified("M"),
        missing = ui::Color::missing("!"),
        rt = ui::Color::maybe_color(ui::color::RESET),
    ));
}

fn version() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}
