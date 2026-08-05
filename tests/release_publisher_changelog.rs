use std::{fs, path::Path, process::Command};

use serde_yaml::Value;

#[test]
#[cfg(unix)]
fn final_publisher_fetches_release_tags_before_generating_notes()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let source = temporary.path().join("source");
    let remote = temporary.path().join("remote.git");
    let checkout = temporary.path().join("checkout");
    fs::create_dir(&source)?;
    run_git(&source, &["init"])?;
    run_git(&source, &["config", "user.email", "codexy@example.com"])?;
    run_git(&source, &["config", "user.name", "Codexy Test"])?;
    fs::write(source.join("release.txt"), "previous\n")?;
    run_git(&source, &["add", "release.txt"])?;
    run_git(&source, &["commit", "-m", "previous release"])?;
    run_git(&source, &["tag", "v1.2.2"])?;
    fs::write(source.join("release.txt"), "published\n")?;
    run_git(&source, &["add", "release.txt"])?;
    run_git(&source, &["commit", "-m", "generated notes commit"])?;
    run_git(&source, &["tag", "v1.3.0"])?;
    let short_sha = git_stdout(&source, &["rev-parse", "--short", "v1.3.0"])?;
    run_git(
        temporary.path(),
        &[
            "clone",
            "--bare",
            source.to_str().ok_or("source")?,
            remote.to_str().ok_or("remote")?,
        ],
    )?;
    run_git(
        temporary.path(),
        &[
            "clone",
            "--no-tags",
            remote.to_str().ok_or("remote")?,
            checkout.to_str().ok_or("checkout")?,
        ],
    )?;

    let generator =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/generate-release-changelog");
    let without_tags = Command::new(&generator)
        .current_dir(&checkout)
        .arg("v1.3.0")
        .output()?;
    assert!(
        !without_tags.status.success(),
        "generator unexpectedly resolved an unfetched tag"
    );

    let release = release_step()?;
    assert!(
        release.contains("git fetch --tags --force origin"),
        "final publisher must fetch release tags before generating notes"
    );
    run_git(&checkout, &["fetch", "--tags", "--force", "origin"])?;
    let output = Command::new(generator)
        .current_dir(&checkout)
        .arg("v1.3.0")
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let notes = String::from_utf8(output.stdout)?;
    assert!(notes.contains("## Codexy v1.3.0"));
    assert!(notes.contains("Changes since v1.2.2:"));
    assert!(notes.contains(&format!("- generated notes commit ({short_sha})")));
    Ok(())
}

fn release_step() -> Result<String, Box<dyn std::error::Error>> {
    let workflow =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Create and verify the only public version release")
        })
        .and_then(|step| step["run"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "final release step".into())
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(cwd).args(args).output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(cwd).args(args).output()?;
    assert!(output.status.success(), "git {args:?} failed");
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
