# deezconfigs

[![license: GPL v3+](https://img.shields.io/badge/license-GPLv3+-blue)](https://www.gnu.org/licenses/gpl-3.0)
![GitHub Tag](https://img.shields.io/github/v/tag/qrichert/deezconfigs?sort=semver&filter=*.*.*&label=release)
[![crates.io](https://img.shields.io/crates/d/deezconfigs?logo=rust&logoColor=white&color=orange)](https://crates.io/crates/deezconfigs)

_Manage deez config files._

> [!CAUTION]
>
> Work In Progress.

Same idea as `GNU Stow` or `chezmoi`, but way simpler, requiring less
neuron activation to operate.

## Yet Another Config Manager

I very rarely edit my configuration files. So when I do, I never quite
remember how the config manager worked. I wanted a tool so easy that
taking 3s to glance at the `--help` would be enough to remember how to
update the configs repo (`deez rsync`), and mirror the changes to my
other environments (`deez sync`).

That's also why `deezconfigs` does very little. Instead of making me
remberer `deez` commands, it delegates to tools I use _way_ more often.
I much rather `nvim` or `git commit` my config files, because those
commands are burnt into my muscle memory.

## Roadmap

- [x] CLI arguments parsing.
- [x] Basic `sync`.
- [x] Protect `$HOME` with a `.deez` file.
- [x] Add `--verbose` mode (use `-V` for version).
- [x] Use Git remote as `sync` root.
- [ ] Basic `rsync`.
- [ ] Basic `link`.
- [ ] Likely `list` (with up-to-date status for each file).
- [ ] Maybe `diff` (difference between source and target).
- [ ] Proper verbose `--help` section.
- [ ] Think about templating.
- [ ] Think about commands.

## Usage

`deez` will copy any file it finds under the `root` to a matching file
in `$HOME`.

- In `sync` mode, the files are copied and the target files overridden.
- `rsync` is the reverse of `sync`, copying from `$HOME` to `root`.
- In `link` mode, symlinks are created to the files in the root.

No special heuristics are planned outside of templates (find a less
intimidating name for that) and variables/secrets.

`deez` requires a `.deez` file in the config root (or it will ask for
confirmation), to prevent yourself from ruining the `$HOME` directory if
ran on the wrong root.

Maybe add `deez diff`, that diffs the two versions (using the `diff`
executable, or settable through an env variable), nothing fancy or
homemade. Maybe there's a crate for this?

```console
$ tree
.
├── .config
│   ├── fish
│   │   └── config.fish
│   ├── ghostty
│   │   ├── config
│   │   └── ssh.txt
│   └── nvim
│       └── init.vim
├── .deez
├── .gitconfig
└── .tmux.conf
```

```console
$ deez --help
deez sync|rsync|link [<root>]

$ deez sync
Copies the files to the $HOME directory.
Also creates any missing directories.

$ deez sync git://git@github.com:qrichert/configs.git
Sync directly from Git remote.

$ deez link
Creates symlinks to the files in $HOME.
Also creates any missing directories.

$ deez sync somedir
Will treat `somedir` as the root instead of using `cwd`.
```

- Respects `.gitignore` (thanks to the `ignore` crate) (TODO: test
  this).

### Tips

#### Do I need to use Git?

Not at all. `deezconfigs` is designed to integrate nicely with Git, but
Git is absolutely not a requirement.

#### Ignore some files

By default, `deezconfigs` ignores the `.git` directory at the root, the
`.gitignore` file at the root (but not elsewhere), and all `.deez`
files, wherever they are (enabling multi-root repos).

If you want to ignore more files than this, add them to your root
`.gitignore`. Git will let you version the files regardless, just
`git add -f` them.

This, in my mind, strikes a nice balance between configurability and
simplicity. You can ignore whatever you want, without squeezing too many
heuristics into `deezconfigs`. It's a Git thing, nothing new to learn.

#### Copying some files, while linking others

Use mutliple roots. You can have multiple roots (subdirectories) in one
repo. Use `sync` in one, and `link` in the other.

I you need anything more advanced than that, `deezconfigs` is likely not
the right tool for you.

### Templating (idea/TODO)

Sometimes you don't want to replace the whole config file, just a part
of it. For example, in `config.fish` you may have generic `fish` config,
but you also want a variable per-machine section.

With `deez` you can achieve this by creating a template section in the
target config file (won't work in `link` mode).

```
# <%deez-template%>

This section will be overridden/updated by the `deez` config file.

# </%deez-template%>

Outside of the template won't be touched.
```

### Commands, aka Hooks (idea/TODO)

Enable some sort of pre-sync and post-sync hooks. Those would be defined
in the `.deez` file. The hooks would be passed to `$SHELL` as:

```sh
$SHELL -c "<the hook content>"
```

This would work with script files (e.g., do-something.bash), and with
commands (e.g., `sed -i s/foo/bar/g`).

The exact format has to be determined. Probably Yaml, like:

```yaml
commands:
  post-sync:
    - do-something.bash
    - sed -i s/foo/bar/g config
```
