use std::path::{Component, Path, PathBuf};

use globset::{Candidate, Glob, GlobBuilder, GlobSet, GlobSetBuilder};

/// A parsed set of include/exclude pathspecs.
///
/// An empty set (no positives and no negatives) means "no filter":
/// everything matches/passes through.
///
/// ## Definitions
///
/// **Pathspecs**: We call _pathspecs_ the full list of 'pathspec'
/// strings we get as input.
///
/// **Pathspec**: We call _pathspec_ the whole string matcher, including
/// the special tokens (i.e., `**/*.toml` or `:!**/*.toml`).
///
/// **Glob**: We call _glob_ the normalized path pattern portion of the
/// pathspec (i.e., `**/*.toml`, without the `:!`)?.
///
/// **Glob Set**: We call _glob set_ a group of globs that can be
/// matched together in a single pass (as per the definition of the
/// [`GlobSet`] struct).
#[derive(Debug, Default)]
pub struct PathSpec {
    positives: GlobSet,
    negatives: GlobSet,
}

impl PathSpec {
    /// Parse raw CLI pathspec strings into a [`PathSpec`].
    ///
    /// A leading `:!` or `:^` makes paths negative (exclude/blacklist);
    /// anything else is positive (include/whitelist).
    ///
    /// # Errors
    ///
    /// Errors if the given pathspec can't match a config file (e.g., it
    /// is empty, absolute, contains `..`, or is a malformed glob).
    pub fn parse(pathspecs: &[String]) -> Result<Self, String> {
        let mut positive_globs = GlobSetBuilder::new();
        let mut negative_globs = GlobSetBuilder::new();

        for pathspec in pathspecs {
            let (is_negative, pattern) = if let Some(rest) = pathspec.strip_prefix(":!") {
                (true, rest)
            } else if let Some(rest) = pathspec.strip_prefix(":^") {
                (true, rest)
            } else {
                (false, pathspec.as_str())
            };

            let pattern = normalize_pattern(pattern)
                .map_err(|reason| format!("Invalid pathspec '{pathspec}': {reason}"))?;

            if is_negative {
                add_glob(&mut negative_globs, pathspec, &pattern)?;
            } else {
                add_glob(&mut positive_globs, pathspec, &pattern)?;
            }
        }

        Ok(Self {
            positives: build_glob_set(&positive_globs)?,
            negatives: build_glob_set(&negative_globs)?,
        })
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positives.is_empty() && self.negatives.is_empty()
    }

    /// Whether `path` (relative to the root) passes the filters.
    ///
    /// If there are filters (pathspecs), `path` must BOTH figure in the
    /// inclusion list AND not figure in the exclusion list. If not
    /// explicitly included OR explicitly excluded, it doesn't match.
    ///
    /// If filters are empty, it always matches (passes).
    #[must_use]
    pub fn matches(&self, path: &Path) -> bool {
        if self.is_empty() {
            return true;
        }
        let candidate = Candidate::new(path);
        let included = self.positives.is_empty() || self.positives.is_match_candidate(&candidate);
        let excluded = self.negatives.is_match_candidate(&candidate);
        included && !excluded
    }
}

fn add_glob(builder: &mut GlobSetBuilder, pathspec: &str, pattern: &str) -> Result<(), String> {
    builder.add(build_glob(pathspec, pattern)?);
    builder.add(build_glob(pathspec, &format!("{pattern}/**"))?);
    Ok(())
}

fn build_glob(pathspec: &str, pattern: &str) -> Result<Glob, String> {
    GlobBuilder::new(pattern)
        .literal_separator(true) // `?` and `*` can never match a path separator.
        .build()
        .map_err(|err| format!("Invalid pathspec '{pathspec}': {err}"))
}

fn build_glob_set(builder: &GlobSetBuilder) -> Result<GlobSet, String> {
    builder
        .build()
        .map_err(|err| format!("Invalid pathspec: {err}"))
}

/// Normalize and validate a raw pathspec pattern.
///
/// Rejects anything that can't denote a concrete _relative_ file or
/// subtree. Since `deezconfigs` runs destructive commands, we want to
/// be conservative, and fail if we're not 100% confident in the give
/// pathspec.
fn normalize_pattern(pattern: &str) -> Result<String, &'static str> {
    if pattern.is_empty() {
        return Err("empty pathspec");
    }

    let mut normalized = PathBuf::new();
    for component in Path::new(pattern).components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            // Drop `.`; a leading `./` is harmless (`./foo` -> `foo`).
            Component::CurDir => {}
            Component::ParentDir => return Err("'..' is not allowed"),
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute paths are not allowed");
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("pathspec normalizes to nothing");
    }

    normalized
        .into_os_string()
        .into_string()
        .map_err(|_| "pathspec is not valid UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_pathspecs(pathspecs: &[&str]) -> PathSpec {
        let pathspecs: Vec<String> = pathspecs.iter().map(ToString::to_string).collect();
        PathSpec::parse(&pathspecs).expect("pathspecs are valid")
    }

    #[test]
    fn empty_spec_matches_everything() {
        let s = parse_pathspecs(&[]);
        assert!(s.is_empty());
        assert!(s.matches(Path::new(".config/fish/config.fish")));
        assert!(s.matches(Path::new("anything")));
    }

    #[test]
    fn positive_exact_file_matches_only_that_file() {
        let s = parse_pathspecs(&[".config/fish/config.fish"]);
        assert!(s.matches(Path::new(".config/fish/config.fish")));
        assert!(!s.matches(Path::new(".config/fish/other.fish")));
        assert!(!s.matches(Path::new(".gitconfig")));
    }

    #[test]
    fn positive_subtree_matches_descendants() {
        let s = parse_pathspecs(&[".config/fish"]);
        assert!(s.matches(Path::new(".config/fish"))); // The dir itself.
        assert!(s.matches(Path::new(".config/fish/config.fish")));
        assert!(s.matches(Path::new(".config/fish/functions/foo.fish")));
        assert!(!s.matches(Path::new(".config/nvim/init.lua")));
    }

    #[test]
    fn multiple_positives_are_or() {
        let s = parse_pathspecs(&[".gitconfig", ".config/nvim"]);
        assert!(s.matches(Path::new(".gitconfig")));
        assert!(s.matches(Path::new(".config/nvim/init.lua")));
        assert!(!s.matches(Path::new(".config/fish/config.fish")));
    }

    #[test]
    fn negation_only_excludes_from_everything() {
        let s = parse_pathspecs(&[":!.config/fish/config.fish"]);
        assert!(!s.is_empty());
        assert!(s.matches(Path::new(".gitconfig")));
        assert!(!s.matches(Path::new(".config/fish/config.fish")));
    }

    #[test]
    fn caret_is_a_negation_alias() {
        let s = parse_pathspecs(&[":^.config/fish"]);
        assert!(!s.matches(Path::new(".config/fish/config.fish")));
        assert!(s.matches(Path::new(".config/nvim/init.lua")));
    }

    #[test]
    fn positive_and_negation_combine() {
        let s = parse_pathspecs(&[".config", ":!.config/fish"]);
        assert!(s.matches(Path::new(".config/nvim/init.lua")));
        assert!(!s.matches(Path::new(".config/fish/config.fish")));
        assert!(!s.matches(Path::new(".gitconfig"))); // Not under `.config`.
    }

    #[test]
    fn normalization_drops_leading_dot_slash_and_trailing_slash() {
        let s = parse_pathspecs(&["./.config/fish/"]);
        assert!(s.matches(Path::new(".config/fish/config.fish")));
    }

    #[test]
    fn star_and_question_mark_stay_within_one_component() {
        let s = parse_pathspecs(&["config?.toml"]);
        assert!(s.matches(Path::new("config1.toml")));
        assert!(!s.matches(Path::new("config12.toml")));
        assert!(!s.matches(Path::new("nested/config1.toml")));

        let s = parse_pathspecs(&["*.toml"]);
        assert!(s.matches(Path::new("config.toml")));
        assert!(s.matches(Path::new(".hidden.toml")));
        assert!(!s.matches(Path::new("nested/config.toml")));
    }

    #[test]
    fn double_star_matches_at_any_depth() {
        let s = parse_pathspecs(&["**/*.toml"]);
        assert!(s.matches(Path::new("config.toml")));
        assert!(s.matches(Path::new("nested/config.toml")));
        assert!(s.matches(Path::new("one/two/config.toml")));
        assert!(!s.matches(Path::new("one/two/config.txt")));
    }

    #[test]
    fn glob_matching_a_directory_includes_its_descendants() {
        let s = parse_pathspecs(&[".config/*"]);
        assert!(s.matches(Path::new(".config/fish/config.fish")));
        assert!(s.matches(Path::new(".config/nvim/lua/plugin.lua")));
        assert!(!s.matches(Path::new("other/fish/config.fish")));
    }

    #[test]
    fn character_classes_and_brace_alternatives_match() {
        let s = parse_pathspecs(&["config[12].{toml,yaml}"]);
        assert!(s.matches(Path::new("config1.toml")));
        assert!(s.matches(Path::new("config2.yaml")));
        assert!(!s.matches(Path::new("config3.toml")));
        assert!(!s.matches(Path::new("config1.json")));
    }

    // These tests pin the glob syntax we document in the README: they
    // assert globset keeps its promises, so an upstream change that
    // alters the matching semantics fails here instead of silently
    // breaking what we advertise.

    #[test]
    fn negated_character_class_excludes_listed_characters() {
        // `[!...]` matches any character _except_ the listed ones.
        let s = parse_pathspecs(&["config[!12].toml"]);
        assert!(s.matches(Path::new("config3.toml")));
        assert!(!s.matches(Path::new("config1.toml")));
        assert!(!s.matches(Path::new("config2.toml")));
    }

    // `\` escapes don't work on Windows where `\` is a path separator.
    #[cfg(not(windows))]
    #[test]
    fn backslash_escapes_a_metacharacter_to_a_literal() {
        // `\*` matches a literal `*`, not "zero or more characters".
        let s = parse_pathspecs(&[r"config\*.toml"]);
        assert!(s.matches(Path::new("config*.toml")));
        assert!(!s.matches(Path::new("config1.toml")));
    }

    // Ensure `\` escapes don't work on Windows and that we don't lie.
    #[cfg(windows)]
    #[test]
    fn backslash_is_a_separator_not_an_escape_on_windows() {
        let s = parse_pathspecs(&[r"config\*.toml"]);
        // `\` acted as a separator: `*` matches a name one level down.
        assert!(s.matches(Path::new("config/sub.toml")));
        // `\` did NOT escape the `*`: a literal `config*.toml` name
        // (a single component, no separator) does not match.
        assert!(!s.matches(Path::new("config*.toml")));
    }

    #[test]
    fn character_class_escapes_a_metacharacter_to_a_literal() {
        // `[*]` is the portable escape (the only one that works on
        // Windows, where `\` is the path separator).
        let s = parse_pathspecs(&["config[*].toml"]);
        assert!(s.matches(Path::new("config*.toml")));
        assert!(!s.matches(Path::new("config1.toml")));
    }

    #[test]
    fn negative_glob_wins_over_positive_glob() {
        let s = parse_pathspecs(&["**/*.toml", ":!**/generated/**"]);
        assert!(s.matches(Path::new("config.toml")));
        assert!(s.matches(Path::new("nested/config.toml")));
        assert!(!s.matches(Path::new("generated/config.toml")));
        assert!(!s.matches(Path::new("nested/generated/config.toml")));
    }

    #[test]
    fn non_prefix_component_does_not_match() {
        // Component-wise: `a/bc` is not under `a/b`.
        let s = parse_pathspecs(&["a/b"]);
        assert!(!s.matches(Path::new("a/bc")));
        assert!(s.matches(Path::new("a/b/c")));
    }

    #[test]
    fn invalid_pathspecs_are_rejected() {
        // Fail closed: never collapse into a match-everything spec.
        for bad in [
            "",
            ":!",
            ":^",
            "/",
            "/foo",
            "..",
            "../foo",
            "foo/../bar",
            ".",
        ] {
            assert!(
                PathSpec::parse(&[bad.to_string()]).is_err(),
                "expected '{bad}' to be rejected"
            );
        }
    }

    #[test]
    fn malformed_glob_is_rejected_with_original_pathspec() {
        let pathspec = String::from(":!config[abc");
        let err = PathSpec::parse(std::slice::from_ref(&pathspec)).unwrap_err();
        assert!(err.contains(&format!("Invalid pathspec '{pathspec}'")));
        assert!(err.contains("unclosed character class"));
    }
}
