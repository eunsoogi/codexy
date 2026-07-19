use std::{fs, path::Path, process::Command};

use super::{counts, scan_source};

pub(crate) fn repository_violations() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    repository_violations_at(Path::new(env!("CARGO_MANIFEST_DIR")))
}

pub(crate) fn repository_violations_at(
    root: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "origin/main", "--", "tests"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err("git diff failed for migration guard".into());
    }
    let mut violations = Vec::new();
    for relative in String::from_utf8(output.stdout)?
        .lines()
        .filter(|path| path.ends_with(".rs"))
    {
        let Some(current) = read_current_source(root, Path::new(relative))? else {
            continue;
        };
        let base = Command::new("git")
            .args(["show", &format!("origin/main:{relative}")])
            .current_dir(root)
            .output()?;
        let base_text = base
            .status
            .success()
            .then(|| String::from_utf8_lossy(&base.stdout));
        let base_violations = base_text.as_deref().map(scan_source).unwrap_or_default();
        let mut allowed = counts(&base_violations);
        for violation in scan_source(&current) {
            let remaining = allowed.entry(violation.clone()).or_default();
            if *remaining == 0 {
                violations.push(format!("{relative}: {violation}"));
            } else {
                *remaining -= 1;
            }
        }
    }
    Ok(violations)
}

pub(crate) fn read_current_source(
    root: &Path,
    relative: &Path,
) -> Result<Option<String>, std::io::Error> {
    match fs::read_to_string(root.join(relative)) {
        Ok(source) => Ok(Some(source)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
