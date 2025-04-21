# deezconfigs

[![license: GPL v3+](https://img.shields.io/badge/license-GPLv3+-blue)](https://www.gnu.org/licenses/gpl-3.0)
![GitHub Tag](https://img.shields.io/github/v/tag/qrichert/deezconfigs?sort=semver&filter=*.*.*&label=release)
[![crates.io](https://img.shields.io/crates/d/deezconfigs?logo=rust&logoColor=white&color=orange)](https://crates.io/crates/deezconfigs)

_Manage deez config files._

_deezconfigs_ will mirror your config files to your `$HOME` directory
(`sync`) and synchronize them back (`rsync`). Additionally, you can
choose to symlink the files instead (`link`).

Same idea as _GNU Stow_ or _chezmoi_, but way simpler, requiring less
neuron activation to operate.

## Usage

```
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

> [!NOTE]
>
> The "config root" can be any directory containing config files. You
> _should_, but are not required to, create a `.deez` file in the root.
> This lets _deezconfigs_ know it is safe to use, and lets you run
> `deez` inside sub-directories.

```console
$ deez sync
Mirror the files in the current root to `$HOME`.
Also create any missing directories.

$ deez sync ~/myconfigs
Use `~/myconfigs` as the config root.

$ deez sync git:git@github.com:qrichert/configs.git
Sync directly from a Git remote (`https://` works too).

$ deez rsync
Sync files from `$HOME` back into the config root.

$ deez link
Create symlinks in `$HOME` instead of copying the files.
Also create any missing directories.

$ deez --help
For more.
```

## Roadmap

- [x] CLI arguments parsing.
- [x] **Command `sync`**.
- [x] Protect `$HOME` with a `.deez` file.
- [x] Add `--verbose` mode (use `-V` for version).
- [x] Use Git remote as `sync` root.
- [x] Smart root finder (looks in parents)
- [x] Hooks (pre, post actions)
- [x] **Command `rsync`**.
- [x] **Command `link`**.
- [x] **Command `status`** (with up-to-date status for each file).
- [x] **Command `clean`**.
- [x] Push files to a `Vec` in verbose mode instead of directly
      printing.
- [x] Pass `DEEZ_ROOT` and `DEEZ_HOME` to hooks.
- [ ] Warn or error if trying to `rsync` `link`ed config. If you do that
      it will empty the config files in the root. So add a check to
      `rsync` that ensures symlinks don't point at configs in root.
- [ ] Handle case where a directory exists where we expect a file (see
      `TODO` comment in all commands).
- [ ] Refactor tests, there is too much duplication (everything `ignore`
      and `walk` can be tested _once_ for all commands).
- [ ] Refactor argument parsing? Maybe?
- [ ] Proper verbose `--help` section.
- [ ] Add hooks examples (maybe even in `--help`).
- [ ] Custom Home directory. Maybe change terminology, Home being the
      default "target".
- [ ] Increase test coverage (features are mostly covered, what's
      missing are tests for the error cases).
- [ ] Perf refactorings for bottlenecks (or for fun).

## FAQ

### Yet Another Config Manager

I very rarely edit my configuration files. So when I do, I never quite
remember how the config manager worked. I wanted a tool so easy that
taking 3s to glance at the `--help` would be enough to remember how to
update the configs repo (`deez rsync`), and mirror the changes to my
other environments (`deez sync`).

That's also why `deezconfigs` does very little. Instead of making me
remberer `deez` commands, it delegates to tools I use _way_ more often.
I much rather `nvim` or `git commit` my config files, because those
commands are burnt into my muscle memory.

### Do I need to use Git?

Not at all. `deezconfigs` is designed to integrate nicely with Git, but
Git is absolutely not a requirement.

### Ignore some files

By default, `deezconfigs` ignores the `.git` directory at the root, the
`.ignore` and/or `.gitignore` file at the root (but not elsewhere,
although it respects them everywhere), all `.deez` files, wherever they
are (enabling multi-root repos), and the hooks (at the root).

If you want to ignore more files than this, add them to your root
`.gitignore`. Git will let you version the files regardless, just
`git add -f` them.

This, in my mind, strikes a nice balance between configurability and
simplicity. You can ignore whatever you want, without squeezing too many
heuristics into `deezconfigs`. It's a Git thing, nothing new to learn.

### Copying some files, while linking others

Use mutliple roots. You can have multiple roots (subdirectories) in one
repo. Use `sync` in one, and `link` in the other.

If you need anything more advanced than that, `deezconfigs` is likely
not the right tool for you.

### No templating?

No. It was an idea at first, but hooks are powerful enough to let you do
your own templating. It's the same idea as "let Git do its thing".
Instead of supporting sub-par templating, _deezconfigs_ defers to hooks.
Nothing's stopping you from using a Python script as a hook with some
Jinja2 template, or any other language/template engine combination you
like.

## Unstructured info dump that needs editing

`deez` requires a `.deez` file in the config root (or it will ask for
confirmation), to prevent yourself from ruining the `$HOME` directory if
ran on the wrong root.

- Respects `.ignore` and `.gitignore` files.
- `list` colors out-of-date files in red (respecting `NO_COLOR`).
- Smart root finding will be used when 1) no root was explicitly
  supplied, and 2) the current working directory (default roor) is not a
  config root (no `.deez` file). In this case, deezconfigs will look
  into parent dirs for a `.deez` file. If one is found, use it as root
  instead of warning "this is not a deez root".

### Hooks

- You can have hooks: `pre-<command>`, `post-<command>`.
- The extension can be any type of script (it's the file name that
  counts).
- The script must be executable and must contain a shebang (`#!`) if not
  interpretable by `sh` directly (e.g., `python` scripts).
- This script will be run through `sh`: `sh -c <root>/<thescript>`
  inside the config root directory.
- Hooks are executed in lexicographic order based on their file name
  (i.e., `post-sync.001.sh` will be run before `post-sync.002.sh`).

_deezconfigs_ provides some basic information to hooks through
environment variables:

- `DEEZ_ROOT` Absolute path to the config Root. This is equal to `pwd`
  on Unix systems, since hooks are run in the root.
- `DEEZ_HOME` Absolute path to the Home directory. This is equal to
  `$HOME` on Unix systems.
- `DEEZ_VERBOSE` Will be `true` if run in verbose mode, otherwise it
  will be unset (hint: use `[ -n $DEEZ_VERBOSE ]` to test for
  existance).
- `DEEZ_OS` Contains the name of the current operating system (e.g,
  `linux`, `macos`, `windows`, etc.) The name is a re-export of Rust's
  [`std::consts::OS`](https://doc.rust-lang.org/std/env/consts/constant.OS.html).
