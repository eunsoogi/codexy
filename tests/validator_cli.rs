use std::process::Command;

#[test]
fn validator_cli_checks_all_contract_surfaces() -> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "--check",
        "--check-mcp",
        "--check-lsp",
        "--check-roles",
        "--print-covered-extensions",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .arg(mode)
            .output()?;
        assert!(
            output.status.success(),
            "validator {mode} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_touched_files_over_loc_target() -> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_fixture(false)?;
    let base = git_head(repo.path())?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path())
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject touched implementation files over 250 LOC"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("src/too_large.rs has 251 lines"),
        "stderr should name the oversized touched file, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_tracked_loc_exception() -> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_fixture(true)?;
    let base = git_head(repo.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "validator should accept tracked LOC exceptions\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_resolves_touched_loc_from_git_root() -> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_fixture(true)?;
    let base = git_head(repo.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path().join("src"))
        .output()?;
    assert!(
        output.status.success(),
        "validator should resolve repo-relative git paths from subdirectories\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_untracked_loc_exception() -> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_fixture(false)?;
    let base = git_head(repo.path())?;
    std::fs::write(
        repo.path().join(".codexy-loc-exceptions"),
        "src/too_large.rs integration harness needs dedicated follow-up split\n",
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path())
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject untracked LOC exception files"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(".codexy-loc-exceptions must be tracked"),
        "stderr should explain the tracked exception requirement, got:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn touched_loc_fixture(
    with_exception: bool,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = temp.path();
    Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["config", "user.email", "codexy@example.test"])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["config", "user.name", "Codexy Test"])
        .current_dir(repo)
        .status()?;
    std::fs::create_dir_all(repo.join("src"))?;
    std::fs::write(repo.join("src/small.rs"), "fn main() {}\n")?;
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["commit", "-qm", "initial"])
        .current_dir(repo)
        .status()?;
    let oversized = (0..251)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect::<String>();
    std::fs::write(repo.join("src/too_large.rs"), oversized)?;
    if with_exception {
        std::fs::write(
            repo.join(".codexy-loc-exceptions"),
            "src/too_large.rs integration harness needs dedicated follow-up split\n",
        )?;
    }
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["commit", "-qm", "add oversized file"])
        .current_dir(repo)
        .status()?;
    Ok(temp)
}

fn git_head(repo: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD~1"])
            .current_dir(repo)
            .output()?
            .stdout,
    )?)
}
