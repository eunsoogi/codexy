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

pub(super) fn validate_inputs() -> Result<()> {
    for path in paths()? {
        let (text, range) = pin(&path)?;
        require_semver(pin_version(&text, &range, &path)?)?;
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
    }
    Ok(())
}

pub(super) fn prepare_version(version: &str) -> Result<Vec<Update>> {
    require_semver(version)?;
    paths()?
        .into_iter()
        .map(|path| {
            let (mut text, range) = pin(&path)?;
            text.replace_range(range, &format!("v{version}"));
            Ok(Update::bytes(path, text.into_bytes()))
        })
        .collect()
}

fn paths() -> Result<Vec<PathBuf>> {
    README_PATHS
        .into_iter()
        .map(|relative| repo_path(relative))
        .collect()
}

fn pin(path: &Path) -> Result<(String, Range<usize>)> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
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
    text[range.clone()].strip_prefix('v').with_context(|| {
        format!(
            "{} marketplace pin must start with v",
            display_relative(path)
        )
    })
}
