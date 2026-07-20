use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn sync_version_cli_checks_manifest_marketplace_parity() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .output()?;
    assert!(
        output.status.success(),
        "sync-version --check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("plugin version sync ok"),
        "unexpected stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

#[test]
fn sync_version_cli_checks_release_tag_parity() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        root.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let version = manifest["version"].as_str().ok_or("manifest version")?;
    let matching_tag = format!("v{version}");
    let matching = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--check", "--tag", &matching_tag])
        .output()?;
    assert!(
        matching.status.success(),
        "matching release tag failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&matching.stdout),
        String::from_utf8_lossy(&matching.stderr)
    );

    let mismatched = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--check", "--tag", "1.1.0"])
        .output()?;
    assert!(
        !mismatched.status.success(),
        "tag without v prefix unexpectedly passed"
    );

    let stale = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--check", "--tag", "v9.9.9"])
        .output()?;
    assert!(
        !stale.status.success(),
        "mismatched release tag unexpectedly passed"
    );
    Ok(())
}

#[test]
fn sync_version_script_check_rejects_stale_cargo_lock_and_stale_python_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp, "repo")?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/sync-plugin-version"),
        repo.join("scripts/sync-plugin-version"),
    )?;

    let lock_path = repo.join("Cargo.lock");
    let lock_text = fs::read_to_string(&lock_path)?;
    let stale_lock = stale_codexy_runtime_lock_version(&lock_text, "9.9.9")?;
    assert_ne!(lock_text, stale_lock, "lock fixture did not change");
    fs::write(&lock_path, stale_lock)?;

    let output = Command::new(repo.join("scripts/sync-plugin-version"))
        .arg("--check")
        .current_dir(&repo)
        .output()?;
    assert!(
        !output.status.success(),
        "sync-version --check unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let after = fs::read_to_string(&lock_path)?;
    assert_eq!(
        stale_codexy_runtime_lock_version(&after, "9.9.9")?,
        after,
        "sync-version --check changed the stale Cargo.lock"
    );

    fs::write(&lock_path, lock_text)?;
    let python_path = repo.join("packages/getcodexy/pyproject.toml");
    let python_text = fs::read_to_string(&python_path)?;
    let version_line = python_text
        .lines()
        .find(|line| line.starts_with("version = "))
        .ok_or("Python package version line")?;
    let stale_python = python_text.replacen(version_line, "version = \"9.9.9\"", 1);
    fs::write(&python_path, &stale_python)?;
    let stale_output = Command::new(repo.join("scripts/sync-plugin-version"))
        .arg("--check")
        .current_dir(&repo)
        .output()?;
    assert!(!stale_output.status.success(), "stale Python metadata passed");
    assert_eq!(fs::read_to_string(&python_path)?, stale_python);
    Ok(())
}

#[test]
fn sync_version_cli_updates_only_the_supplied_isolated_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let build_root = archive_repository(&temp, "build-root")?;
    let diagnostic_root = archive_repository(&temp, "diagnostic-root")?;
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/paths.rs"),
        build_root.join("src/paths.rs"),
    )?;
    let build_target = build_root.join("target");

    let build_status = Command::new("cargo")
        .args([
            "build",
            "--locked",
            "--quiet",
            "--bin",
            "codexy-sync-version",
        ])
        .env("CARGO_TARGET_DIR", &build_target)
        .current_dir(&build_root)
        .status()?;
    assert!(build_status.success(), "isolated helper build failed");

    let build_root_before = version_surface_contents(&build_root)?;
    let output = Command::new(build_target.join("debug/codexy-sync-version"))
        .args(["--version", "9.9.9"])
        .env("CODEXY_REPO_ROOT", &diagnostic_root)
        .current_dir(&diagnostic_root)
        .output()?;
    assert!(
        output.status.success(),
        "isolated diagnostic failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        version_surface_contents(&build_root)?,
        build_root_before,
        "the compiled helper mutated its baked-in build root"
    );
    for (path, contents) in version_surface_contents(&diagnostic_root)? {
        let text = String::from_utf8_lossy(&contents);
        assert!(
            text.lines().map(str::trim).any(|line| matches!(
                line,
                "version = \"9.9.9\"" | "\"version\": \"9.9.9\","
            )),
            "supplied diagnostic root was not updated at {}",
            path.display()
        );
    }
    Ok(())
}

fn archive_repository(
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
        "packages/getcodexy/pyproject.toml",
        "src/version.rs",
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

fn version_surface_contents(
    root: &Path,
) -> Result<Vec<(PathBuf, Vec<u8>)>, Box<dyn std::error::Error>> {
    [
        ".agents/plugins/marketplace.json",
        ".agents/plugins/release-publish-contract.json",
        "Cargo.lock",
        "Cargo.toml",
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

fn stale_codexy_runtime_lock_version(
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
