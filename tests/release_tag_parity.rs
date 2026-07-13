use std::{fs, path::Path, process::Command};

#[test]
fn rejects_each_synchronized_source_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp)?;
    let binary = Path::new(env!("CARGO_BIN_EXE_codexy-sync-version"));
    let manifest: serde_json::Value = serde_json::from_str(&fs::read_to_string(
        repo.join("plugins/codexy/.codex-plugin/plugin.json"),
    )?)?;
    let version = manifest["version"].as_str().ok_or("fixture version")?;
    let tag = format!("v{version}");
    let replacement = if version == "9.9.9" { "8.8.8" } else { "9.9.9" };
    for relative in [
        "plugins/codexy/.codex-plugin/plugin.json",
        ".agents/plugins/marketplace.json",
        ".agents/plugins/release-publish-contract.json",
        "Cargo.toml",
        "Cargo.lock",
    ] {
        let path = repo.join(relative);
        let original = fs::read_to_string(&path)?;
        let mutated = if relative == "Cargo.lock" {
            stale_runtime_lock(&original, replacement)
        } else if relative.ends_with(".json") {
            original.replacen(
                &format!("\"version\": \"{version}\""),
                &format!("\"version\": \"{replacement}\""),
                1,
            )
        } else {
            original.replacen(
                &format!("version = \"{version}\""),
                &format!("version = \"{replacement}\""),
                1,
            )
        };
        assert_ne!(original, mutated, "fixture did not mutate {relative}");
        fs::write(&path, mutated)?;
        let output = Command::new(binary)
            .args(["--check", "--tag", &tag])
            .env("CODEXY_REPO_ROOT", &repo)
            .output()?;
        assert!(
            !output.status.success(),
            "mismatched {relative} unexpectedly passed"
        );
        fs::write(path, original)?;
    }
    Ok(())
}

fn archive_repository(
    temp: &tempfile::TempDir,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let archive = temp.path().join("repo.tar");
    let repo = temp.path().join("repo");
    assert!(
        Command::new("git")
            .args(["archive", "--format=tar", "HEAD", "-o"])
            .arg(&archive)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()?
            .success()
    );
    fs::create_dir(&repo)?;
    assert!(
        Command::new("tar")
            .args(["-xf"])
            .arg(&archive)
            .arg("-C")
            .arg(&repo)
            .status()?
            .success()
    );
    Ok(repo)
}

fn stale_runtime_lock(text: &str, replacement: &str) -> String {
    let mut runtime = false;
    text.lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed == "[[package]]" {
                runtime = false;
            }
            if trimmed == "name = \"codexy-runtime\"" {
                runtime = true;
            }
            if runtime && trimmed.starts_with("version = ") {
                runtime = false;
                format!("version = \"{replacement}\"")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
