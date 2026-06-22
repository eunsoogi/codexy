use std::process::Command;

#[test]
fn infers_previous_tag_from_release_history() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir()?;

    run_git(temp.path(), &["init"])?;
    run_git(temp.path(), &["config", "user.email", "codexy@example.com"])?;
    run_git(temp.path(), &["config", "user.name", "Codexy Test"])?;

    std::fs::write(temp.path().join("file.txt"), "base\n")?;
    run_git(temp.path(), &["add", "file.txt"])?;
    run_git_at(
        temp.path(),
        &["commit", "-m", "base release"],
        "2026-01-01T00:00:00Z",
    )?;
    run_git(temp.path(), &["tag", "v0.1.0"])?;

    run_git(temp.path(), &["switch", "-c", "backport", "v0.1.0"])?;
    std::fs::write(temp.path().join("file.txt"), "backport fix\n")?;
    run_git(temp.path(), &["add", "file.txt"])?;
    run_git_at(
        temp.path(),
        &["commit", "-m", "backport fix"],
        "2026-02-01T00:00:00Z",
    )?;
    run_git(temp.path(), &["tag", "v0.1.1"])?;

    run_git(temp.path(), &["switch", "-c", "mainline", "v0.1.0"])?;
    std::fs::write(temp.path().join("file.txt"), "newer mainline\n")?;
    run_git(temp.path(), &["add", "file.txt"])?;
    run_git_at(
        temp.path(),
        &["commit", "-m", "newer mainline release"],
        "2026-03-01T00:00:00Z",
    )?;
    run_git(temp.path(), &["tag", "v0.2.0"])?;

    let output = Command::new(root.join("scripts/generate-release-changelog"))
        .current_dir(temp.path())
        .arg("v0.1.1")
        .output()?;

    assert!(
        output.status.success(),
        "script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Changes since v0.1.0:"),
        "omitted previous tag must come from release history, not newest repository tag:\n{stdout}"
    );
    assert!(stdout.contains("- backport fix ("));
    assert!(
        !stdout.contains("newer mainline release"),
        "changelog must not include commits outside the release history:\n{stdout}"
    );
    Ok(())
}

fn run_git(cwd: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(cwd).args(args).output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn run_git_at(
    cwd: &std::path::Path,
    args: &[&str],
    date: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .current_dir(cwd)
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .args(args)
        .output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
