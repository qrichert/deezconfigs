use std::path::{Component, Path, PathBuf};

/// A parsed set of include/exclude pathspecs.
///
/// Empty (no positives and no negatives) means "no filter": everything
/// matches/passes through.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct PathSpec {
    positives: Vec<PathBuf>,
    negatives: Vec<PathBuf>,
}

impl PathSpec {
    /// Parse raw CLI pathspec strings into a [`PathSpec`].
    ///
    /// A leading `:!` or `:^` makes a token negative (exclude);
    /// anything else is positive (include).
    ///
    /// # Errors
    ///
    /// Errors if the given pathspec string can't possibly match a
    /// config file (e.g., empty strings, absolute paths, etc.).
    /// See `normalize()`.
    pub fn parse(tokens: &[String]) -> Result<Self, String> {
        let mut positives = Vec::new();
        let mut negatives = Vec::new();

        for token in tokens {
            let (is_negative, raw) = if let Some(rest) = token.strip_prefix(":!") {
                (true, rest)
            } else if let Some(rest) = token.strip_prefix(":^") {
                (true, rest)
            } else {
                (false, token.as_str())
            };

            let spec =
                normalize(raw).map_err(|reason| format!("Invalid pathspec '{token}': {reason}"))?;

            if is_negative {
                negatives.push(spec);
            } else {
                positives.push(spec);
            }
        }

        Ok(Self {
            positives,
            negatives,
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
        let included =
            self.positives.is_empty() || self.positives.iter().any(|s| path.starts_with(s));
        let excluded = self.negatives.iter().any(|s| path.starts_with(s));
        included && !excluded
    }
}

/// Normalize (and validate) a raw pathspec string.
///
/// Rejects anything that can't denote a concrete _relative_ file or
/// subtree. Since `deezconfigs` runs destructive commands, we want to
/// be conservative, and fail if we're not 100% confident in the give
/// pathspec.
fn normalize(raw: &str) -> Result<PathBuf, &'static str> {
    if raw.is_empty() {
        return Err("empty pathspec");
    }

    let mut normalized = PathBuf::new();
    for component in Path::new(raw).components() {
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

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_pathspecs(tokens: &[&str]) -> PathSpec {
        let tokens: Vec<String> = tokens.iter().map(ToString::to_string).collect();
        PathSpec::parse(&tokens).expect("tokens are valid")
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
    fn non_prefix_component_does_not_match() {
        // Component-wise: `a/bc` is not under `a/b`.
        let s = parse_pathspecs(&["a/b"]);
        assert!(!s.matches(Path::new("a/bc")));
        assert!(s.matches(Path::new("a/b/c")));
    }

    #[test]
    fn invalid_tokens_are_rejected() {
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
}
