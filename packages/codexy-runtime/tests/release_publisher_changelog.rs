use std::{fs, path::Path, process::Command};

use crate::support;

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
        codexy_runtime::paths::repository_root().join("scripts/generate-release-changelog");
    let without_tags = Command::new(&generator)
        .current_dir(&checkout)
        .arg("v1.3.0")
        .output()?;
    assert!(
        !without_tags.status.success(),
        "generator unexpectedly resolved an unfetched tag"
    );

    let release = release_step()?;
    support::assert_structured_literals(
        &release,
        "final publisher release tag fetch",
        &["git fetch --tags --force origin"],
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
    let generated_commit = format!("- generated notes commit ({short_sha})");
    support::assert_structured_literals(
        &notes,
        "generated changelog notes",
        &["## Codexy v1.3.0", "Changes since v1.2.2:", &generated_commit],
    );
    Ok(())
}

#[test]
fn final_publisher_keeps_changelog_materialization_readable() -> Result<(), Box<dyn std::error::Error>> {
    let release = release_step()?;
    support::assert_structured_literals(
        &release,
        "readable final publisher changelog materialization",
        &[
            "release_exists=false",
            "if gh release view \"$RELEASE_TAG\" --repo \"$GITHUB_REPOSITORY\" --json id,name,tagName,targetCommitish,isDraft,isPrerelease,assets > release-state.json 2>/dev/null; then",
            "if test \"$release_exists\" = false; then\n  changelog_notes=\"$(scripts/generate-release-changelog \"$RELEASE_TAG\")\"",
        ],
    );
    support::assert_structured_absent_literals(
        &release,
        "readable final publisher changelog materialization",
        &["then changelog_notes="],
    );
    Ok(())
}

fn release_step() -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"),
    )?)
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
