//! Checks and updates marketplace and bootstrap CLI pins in both project READMEs.

use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

use super::{mutation::Update, repo_path, require_matching_version, require_semver};

const README_PATHS: [&str; 2] = ["README.md", "README.ko.md"];
const PIN_PREFIX: &str = "codex plugin marketplace add eunsoogi/codexy --ref ";
const BOOTSTRAP_PREFIXES: [&str; 2] =
    ["uv tool install getcodexy==", "uv tool upgrade getcodexy=="];

pub(super) fn validate_inputs() -> Result<()> {
    for path in paths()? {
        let (text, range) = pin(&path)?;
        require_semver(pin_version(&text, &range, &path)?)?;
        bootstrap_ranges(&text, &path)?;
    }
    Ok(())
}

pub(super) fn check(expected: &str) -> Result<()> {
    for path in paths()? {
        let (text, range) = pin(&path)?;
        let actual = pin_version(&text, &range, &path)?;
        require_matching_version(
            actual,
            &display_relative(&path),
            expected,
            "version sync target",
        )?;
        for range in bootstrap_ranges(&text, &path)? {
            require_matching_version(
                &text[range],
                &display_relative(&path),
                expected,
                "version sync target",
            )?;
        }
    }
    Ok(())
}

pub(super) fn prepare_version(version: &str) -> Result<Vec<Update>> {
    require_semver(version)?;
    paths()?
        .into_iter()
        .map(|path| {
            let (mut text, range) = pin(&path)?;
            pin_version(&text, &range, &path)?;
            let mut ranges = bootstrap_ranges(&text, &path)?;
            // Replace from the end so earlier byte offsets remain valid, regardless of command order.
            ranges.push(range.start + 1..range.end);
            ranges.sort_by_key(|range| std::cmp::Reverse(range.start));
            for range in ranges {
                text.replace_range(range, version);
            }
            Ok(Update::bytes(path, text.into_bytes()))
        })
        .collect()
}

fn paths() -> Result<Vec<PathBuf>> {
    README_PATHS.into_iter().map(repo_path).collect()
}

fn pin(path: &Path) -> Result<(String, Range<usize>)> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    // Match the executable install command exactly so unrelated version prose is untouched.
    let matches = text.match_indices(PIN_PREFIX).collect::<Vec<_>>();
    if matches.len() != 1 {
        bail!(
            "{} must contain exactly one direct marketplace pin",
            display_relative(path)
        );
    }
    let start = matches[0].0 + PIN_PREFIX.len();
    let end = text[start..]
        .find(char::is_whitespace)
        .map_or(text.len(), |offset| start + offset);
    if start == end {
        bail!("{} marketplace pin is empty", display_relative(path));
    }
    Ok((text, start..end))
}

fn pin_version<'a>(text: &'a str, range: &Range<usize>, path: &Path) -> Result<&'a str> {
    let version = text[range.clone()].strip_prefix('v').with_context(|| {
        format!(
            "{} marketplace pin must start with v",
            display_relative(path)
        )
    })?;
    require_semver(version).with_context(|| {
        format!(
            "{} marketplace pin has an invalid version",
            display_relative(path)
        )
    })?;
    Ok(version)
}

// Exactly one install and upgrade command is required; prose and historical examples are untouched.
fn bootstrap_ranges(text: &str, path: &Path) -> Result<Vec<Range<usize>>> {
    BOOTSTRAP_PREFIXES
        .into_iter()
        .map(|prefix| {
            // Inline migration examples are historical prose, not executable command lines.
            let matches = text
                .match_indices(prefix)
                .filter(|(start, _)| {
                    let line_start = text[..*start].rfind('\n').map_or(0, |offset| offset + 1);
                    text[line_start..*start].trim().is_empty()
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                bail!(
                    "{} must contain exactly one {prefix} pin",
                    display_relative(path)
                );
            }
            let start = matches[0].0 + prefix.len();
            let end = text[start..]
                .find(char::is_whitespace)
                .map_or(text.len(), |offset| start + offset);
            require_semver(&text[start..end]).with_context(|| {
                format!(
                    "{} {prefix} pin has an invalid version",
                    display_relative(path)
                )
            })?;
            Ok(start..end)
        })
        .collect()
}
