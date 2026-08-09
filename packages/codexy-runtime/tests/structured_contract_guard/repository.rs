use std::{collections::HashMap, fs, path::Path, process::Command};

use super::{governed_assertions, scan_source, GovernedAssertion};

pub(crate) fn comparison_counts(
    relative_paths: &[&str],
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    comparison_counts_at(&codexy_runtime::paths::runtime_package_root(), relative_paths)
}

pub(crate) fn repository_violations() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    repository_violations_at(&codexy_runtime::paths::runtime_package_root())
}

pub(crate) fn repository_violations_at(
    root: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let paths = RepositoryPaths::at(root)?;
    let merge_base = merge_base(root)?;
    let output = Command::new("git")
        .args([
            "diff",
            "--diff-filter=d",
            "-z",
            "--name-only",
            &merge_base,
            "--",
            &paths.repository_relative("tests")?,
        ])
        .current_dir(&paths.top_level)
        .output()?;
    if !output.status.success() {
        return Err("git diff failed for migration guard".into());
    }

    let mut violations = Vec::new();
    for relative in String::from_utf8(output.stdout)?
        .split('\0')
        .filter(|path| !path.is_empty())
        .filter(|path| path.ends_with(".rs"))
    {
        let current_path = paths.working_path(relative)?;
        if !current_path.is_file() {
            continue;
        }
        let current = fs::read_to_string(current_path)?;
        let base_assertions = source_at(root, &paths, &merge_base, relative)?
            .as_deref()
            .map(governed_assertions)
            .unwrap_or_default();
        let mut allowed = counts(&base_assertions);
        for assertion in governed_assertions(&current) {
            let remaining = allowed.entry(assertion.identity).or_default();
            if *remaining == 0 {
                violations.push(format!("{}: {}", paths.display_relative(relative)?, assertion.diagnostic));
            } else {
                *remaining -= 1;
            }
        }
    }
    Ok(violations)
}

pub(crate) fn comparison_counts_at(
    root: &Path,
    relative_paths: &[&str],
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let paths = RepositoryPaths::at(root)?;
    let merge_base = merge_base(root)?;
    let mut before = 0;
    let mut after = 0;
    for relative in relative_paths {
        let current = fs::read_to_string(paths.working_path(relative)?)?;
        let base = source_at(root, &paths, &merge_base, relative)?
            .ok_or_else(|| format!("missing {merge_base}:{relative}"))?;
        before += scan_source(&base).len();
        after += scan_source(&current).len();
    }
    Ok((before, after))
}

fn merge_base(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["merge-base", "origin/main", "HEAD"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("git merge-base failed for migration guard".into());
    }
    let merge_base = String::from_utf8(output.stdout)?.trim().to_owned();
    if merge_base.is_empty() {
        return Err("git merge-base returned an empty revision".into());
    }
    Ok(merge_base)
}

fn source_at(
    root: &Path,
    paths: &RepositoryPaths,
    revision: &str,
    relative: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["show", &format!("{revision}:{}", paths.repository_relative(relative)?)])
        .current_dir(root)
        .output()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout))
        .transpose()
        .map_err(Into::into)
}

struct RepositoryPaths {
    top_level: std::path::PathBuf,
    prefix: std::path::PathBuf,
}

impl RepositoryPaths {
    fn at(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel", "--show-prefix"])
            .current_dir(root)
            .output()?;
        if !output.status.success() {
            return Err("git repository path lookup failed for migration guard".into());
        }
        let output = String::from_utf8(output.stdout)?;
        let (top_level, prefix) = output
            .split_once('\n')
            .ok_or("git repository path lookup returned incomplete output")?;
        let top_level = std::path::PathBuf::from(top_level.trim_end_matches('\r'));
        let prefix = std::path::PathBuf::from(prefix.trim_end_matches(['\r', '\n']));
        if !top_level.is_absolute() || !safe_relative(&prefix) {
            return Err("git repository path lookup returned an unsafe path".into());
        }
        Ok(Self { top_level, prefix })
    }

    fn repository_relative(&self, relative: &str) -> Result<String, Box<dyn std::error::Error>> {
        let relative = Path::new(relative);
        if !safe_relative(relative) {
            return Err("migration guard path must be relative and normalized".into());
        }
        let path = if relative.starts_with(&self.prefix) {
            relative.to_path_buf()
        } else {
            self.prefix.join(relative)
        };
        Ok(path.to_string_lossy().into_owned())
    }

    fn working_path(&self, relative: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        Ok(self.top_level.join(self.repository_relative(relative)?))
    }

    fn display_relative(&self, relative: &str) -> Result<String, Box<dyn std::error::Error>> {
        let path = std::path::PathBuf::from(self.repository_relative(relative)?);
        Ok(path
            .strip_prefix(&self.prefix)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned())
    }
}

fn safe_relative(path: &Path) -> bool {
    path.components().all(|component| matches!(component, std::path::Component::Normal(_) | std::path::Component::CurDir))
}

fn counts(assertions: &[GovernedAssertion]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for assertion in assertions {
        *counts.entry(assertion.identity.clone()).or_default() += 1;
    }
    counts
}
