# deezconfigs

[![license: GPL v3+](https://img.shields.io/badge/license-GPLv3+-blue)](https://www.gnu.org/licenses/gpl-3.0)
![GitHub Tag](https://img.shields.io/github/v/tag/qrichert/deezconfigs?sort=semver&filter=*.*.*&label=release)
[![tokei (loc)](https://tokei.rs/b1/github/qrichert/deezconfigs?label=loc&style=flat)](https://github.com/XAMPPRocky/tokei)
[![crates.io](https://img.shields.io/crates/d/deezconfigs?logo=rust&logoColor=white&color=orange)](https://crates.io/crates/deezconfigs)
[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/qrichert/deezconfigs/ci.yml?label=tests)](https://github.com/qrichert/deezconfigs/actions)

_Manage deez config files._

_deezconfigs_ will mirror your config files to your `$HOME` directory
(`sync`) and synchronize them back (`rsync`). Additionally, you can
choose to symlink the files instead (`link`).

Same idea as _GNU Stow_ or _chezmoi_, but simpler, requiring less neuron
activation to operate.

## Get `--help`

```
Manage deez config files.

Usage: deez [<options>] <command> [<args>]

Commands:
  sync [<root>|<git>]    Update Home from configs
  rsync [<root>]         Update configs from Home
  link [<root>]          Symlink configs to Home

  status [<root>|<git>]  List files and their status
  diff [<root>|<git>]    Show what has changed
  clean [<root>|<git>]   Remove all configs from Home

Options:
  -h, --help             Show this message and exit
  -V, --version          Show the version and exit
  -v, --verbose          Show files being copied
```

### What does deezconfigs do?

The core of deezconfigs is to replicate the file structure of a given
directory (i.e., the config root), inside the Home. The main purpose of
this is to keep all the config files in one place, making it easy to
version them.

deezconfigs is very un-opinionated by default. It tries to do its job
well (syncing config files), while avoiding to do what other tools do
better. For instance, there is no automatic versioning, no embedded text
editor, and no templating. You absolutely _can_ do all of the above, but
it's not something that's forced on you. It's _your_ processes, _your_
tools. All the extensibility power lies in hooks (read further below).

### Copying vs. Linking

deezconfigs supports two configuration models: copying and linking. Both
models come with different trade-offs. For instance, linking ensures
files are always up-to-date, but on the flip-side, you can't really have
machine specific configuration. On the other hand, copying files need to
be kept up-to-date manually by `sync`ing or `rsync`ing all changes. But,
having separate copies makes it easier to keep configuration generic in
the root, and specific in the Home.

### The Config Root

As mentioned before, the config root is any directory whose structure
you want to replicate in the Home directory.

That said, you _should_, but are not required to, create a `.deez` file
in the root. This lets deezconfigs know it is safe to use. If
deezconfigs doesn't find a `.deez` file, it will ask you confirmation
before modifying you file system. This is a security feature to prevent
you from accidentally messing up your Home if you run `deez` from the
wrong directory.

Another advantage of creating a proper root is that that it lets you run
`deez` inside sub-directories as well. Just like you can run Git
commands from anywhere in the repo, deezconfigs is smart enough to
search for a root in parent directories before warning you that the
current directory is not a root.

### Home

This is the directory where config files are copied or symlinked to. On
Unix, this is read from the `HOME` environment variable, and on Windows
from `USERPROFILE`.

Using a different Home is not natively supported by an argumment, but
you can override the environment variable to achieve what you want.

```console
$ HOME=/home/other deez sync
```

### Sync

Syncing in deezconfigs replicates the file structure from the config
root inside the Home directory (minus ignored files).

```console
# Sync current config root.
$ deez sync

# Sync given config root, verbosely.
$ deez --verbose sync ~/configs

# Sync from remote.
$ deez sync https://github.com/qrichert/configs
```

### rSync

Reverse-syncing is the complimentary opposite of syncing: it updates
your config files in the root with the current content from Home.

```console
# 1. Sync your config file to your Home.
$ deez sync

# 2. Make some changes.
$ vim ~/.gitconfig

# 3. rSync the changes back into your root.
$ deez rsync
```

### Link

Linking is the same as syncing, but it creates symbolic links in the
Home instead of copying files. Linking has no `rsync` equivalent because
linked files are always up-to-date.

```console
# Symlink current config root.
$ deez link
```

### Status

Status prints the list of configuration files with their respective
state of 'syncness', and also prints your hooks.

Configuration files can be:

```
S  In Sync
M  Modified
!  Missing
```

### Diff

Diffing prints the line-diff between your config root and your Home.
This shows you exactly what has changed and where. There is no merge
feature however, as merging is best done by your VCS.

By default, `diff` uses the config root as the _before_, and the Home as
the _after_. This assumes you make changes in the Home directly, and
want to see what would change in your root if you `rsync`ed the updates
back.

```console
# Compare the config root (old) to the Home (new).
$ deez diff
```

If you make changes inside the config root however, it is more natural
to use the Home as the _before_, and the root as the _after_. In other
words, you want to see what would change in your Home if you `sync`ed
the updates to it.

To do this, use the `--reversed` flag:

```console
# Compare the Home (old) to the config root (new).
$ deez diff -r
```

### Clean

Cleaning is removing all the files and symlinks from the Home.

```console
# 1. Link your files to your Home.
$ deez link

# 2. Now remove all the links you've just created.
$ deez clean
```

### Shortcuts

Each command has a shortcut:

```
sync   s     status  st
rsync  rs    diff    df
link   l     clean   c
```

### Ignore some files

By default, deezconfigs ignores all the hook files (at the root) the
`.git` directory at the root (if any), all `.ignore` and `.gitignore`
files, and all `.deez` files, wherever they are (enabling multi-root
repos).

You can extend this list by adding entries to your `.ignore` and/or
`.gitignore` files, they are both respected by deezconfigs.

If you want to both version a file in Git and have it ignored by
deezconfigs, you can either add it to a `.gitignore` and `git add -f`
it, or you can use a generic `.ignore` file instead.

### Git

Git is optional, but deezconfigs is designed to integrate nicely with
it. Beyond respecting `.gitignore` files, deezconfigs can use any Git
remote as config root with `sync`, `status`, `diff` and `clean`.

To expand on a previous example:

```console
# Sync from remote.
$ deez sync https://github.com/qrichert/configs
```

This will clone the repository to a temporary directory behind the
scenes, and update your Home with its contents. This is useful in places
where you don't want to maintain a proper clone, and always just want to
get the latest version.

deezconfigs considers a Git root any root starting with either `git:`,
`ssh://`, `git@`, `https://`, or `http://`. `git:` is a special label
you can use to force a path to be considered a Git root.

In addition, `gh:` will be replaced with `git@github.com:`, (e.g.,
`gh:qrichert/configs`).

### Hooks

deezconfigs let you run hooks before and after commands. Hooks are
scripts or executables located at the root and whose names match the
following pattern:

```
(pre|post)-<command>[.extension]
```

A common example would be...

```
post-sync.sh
```

...a shell script that gets run after every `deez sync` command.

You can have multiple hooks for the same action; they will be run in
name order (`post-sync.001.sh`, then `post-sync.002.sh`, etc.).

Hooks are executed through `sh`. It is roughly equivalent to:

```console
$ cd <root>
$ export DEEZ_...  # deez envionrment variables.
$ sh -c "<root>/<hook>"
```

Note that you'll likely want the scripts to start with a shebang (e.g.,
`#!/usr/bin/env python3`).

As an example, here are two complimentary scripts that respectively set
and unset Git's email address in the `.gitconfig` file when you `sync`
and `rsync` it:

```console
$ cat post-sync.sh
#!/usr/bin/env bash
[[ -n $DEEZ_VERBOSE ]] && echo "Set global Git email address."
git config --global user.email you@example.com

$ cat post-rsync.sh
#!/usr/bin/env bash
[[ -n $DEEZ_VERBOSE ]] && echo "Unset Git email address."
git config --file ./.gitconfig user.email '<>'
```

They both make use of the `DEEZ_VERBOSE` environment variable to enrich
the output of `deez` in verbose mode.

deezconfigs passes a few envionrment variables to hooks to make your
life easier:

- `DEEZ_ROOT` Absolute path to the config Root. This is equal to `pwd`
  on Unix systems, since hooks are run in the root.
- `DEEZ_HOME` Absolute path to the Home directory. This is equal to
  `$HOME` on Unix systems.
- `DEEZ_VERBOSE` Will be `true` if run in verbose mode, otherwise it
  will be unset (hint: use `[[ -n $DEEZ_VERBOSE ]]` to test for
  existance).
- `DEEZ_OS` Contains the name of the current operating system (e.g,
  `linux`, `macos`, `windows`, etc.). The name is a re-export of Rust's
  [`std::consts::OS`].

[`std::consts::OS`]:
  https://doc.rust-lang.org/std/env/consts/constant.OS.html

### Templating

There is no built-in templating in deezconfigs, but you can implement
simple to very tailored templating with hooks. From simple `sed`
commands, to something way more advanced like Jinja2 in Python.

### Copy some files, and link others

Use mutliple roots. You can have multiple roots (subdirectories) in one
repo. Use `sync` in one, and `link` in the other.

If you need anything more advanced than that, `deezconfigs` is likely
not the right tool for you.

## Roadmap

- [x] **Command `sync`**.
- [x] **Command `rsync`**.
- [x] **Command `link`**.
- [x] **Command `status`**.
- [x] **Command `diff`**.
- [x] **Command `clean`**.
- [ ] Enable subroots with remotes (`git@gh.com/user/repo[sub/root]`).
- [ ] Allow syncing a single file if root points to a file.
- [ ] Increase test coverage (features are mostly covered, what's
      missing are tests for the error cases).
- [ ] Perf refactorings for bottlenecks (or for fun).
