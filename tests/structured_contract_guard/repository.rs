use std::{collections::HashMap, fs, path::Path, process::Command};

use super::{governed_assertions, scan_source, GovernedAssertion};

pub(crate) fn comparison_counts(
    relative_paths: &[&str],
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    comparison_counts_at(Path::new(env!("CARGO_MANIFEST_DIR")), relative_paths)
}

pub(crate) fn repository_violations() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    repository_violations_at(Path::new(env!("CARGO_MANIFEST_DIR")))
}

pub(crate) fn repository_violations_at(
    root: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let merge_base = merge_base(root)?;
    let output = Command::new("git")
        .args([
            "diff",
            "--diff-filter=d",
            "-z",
            "--name-only",
            &merge_base,
            "--",
            "tests",
        ])
        .current_dir(root)
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
        let current_path = root.join(relative);
        if !current_path.is_file() {
            continue;
        }
        let current = fs::read_to_string(current_path)?;
        let base_assertions = source_at(root, &merge_base, relative)?
            .as_deref()
            .map(governed_assertions)
            .unwrap_or_default();
        let mut allowed = counts(&base_assertions);
        for assertion in governed_assertions(&current) {
            let remaining = allowed.entry(assertion.identity).or_default();
            if *remaining == 0 {
                violations.push(format!("{relative}: {}", assertion.diagnostic));
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
    let merge_base = merge_base(root)?;
    let mut before = 0;
    let mut after = 0;
    for relative in relative_paths {
        let current = fs::read_to_string(root.join(relative))?;
        let base = source_at(root, &merge_base, relative)?
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
    revision: &str,
    relative: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["show", &format!("{revision}:{relative}")])
        .current_dir(root)
        .output()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout))
        .transpose()
        .map_err(Into::into)
}

fn counts(assertions: &[GovernedAssertion]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for assertion in assertions {
        *counts.entry(assertion.identity.clone()).or_default() += 1;
    }
    counts
}
