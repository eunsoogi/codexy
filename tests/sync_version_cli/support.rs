use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn archive_repository(
    temp: &tempfile::TempDir,
    name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let archive = temp.path().join(format!("{name}.tar"));
    let repo = temp.path().join(name);
    let archive_status = Command::new("git")
        .args(["archive", "--format=tar", "HEAD"])
        .arg("-o")
        .arg(&archive)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;
    assert!(archive_status.success(), "git archive failed");
    fs::create_dir(&repo)?;
    let tar_status = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&repo)
        .status()?;
    assert!(tar_status.success(), "tar extract failed");
    for relative in [
        "install",
        "packages/getcodexy/pyproject.toml",
        "src/version.rs",
        "src/version/install.rs",
        "src/version/package.rs",
        "src/version/python.rs",
    ] {
        let destination = repo.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative), destination)?;
    }
    Ok(repo)
}

pub(super) fn version_surface_contents(
    root: &Path,
) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn std::error::Error>> {
    [
        ".agents/plugins/marketplace.json",
        ".agents/plugins/release-publish-contract.json",
        "Cargo.lock",
        "Cargo.toml",
        "install",
        "packages/getcodexy/pyproject.toml",
        "plugins/codexy/.codex-plugin/plugin.json",
    ]
    .into_iter()
    .map(|relative| {
        let path = root.join(relative);
        Ok((path.clone(), fs::read(path)?))
    })
    .collect()
}

pub(super) fn stale_codexy_runtime_lock_version(
    lock_text: &str,
    stale_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut in_codexy_runtime = false;
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in lock_text.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_codexy_runtime = false;
        } else if trimmed == "name = \"codexy-runtime\"" {
            in_codexy_runtime = true;
        }

        if in_codexy_runtime && trimmed.starts_with("version = ") {
            lines.push(format!("version = \"{stale_version}\""));
            replaced = true;
            in_codexy_runtime = false;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        return Err("codexy-runtime package version not found in Cargo.lock".into());
    }
    Ok(format!("{}\n", lines.join("\n")))
}
