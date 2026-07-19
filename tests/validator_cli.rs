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
fn validator_cli_rejects_oversized_files_despite_tracked_loc_exception()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_fixture(true)?;
    let base = git_head(repo.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path())
        .output()?;
    assert!(
        !output.status.success(),
        "validator should reject oversized files despite a tracked LOC exception\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("src/too_large.rs has 251 lines"));
    assert!(stderr.contains(".codexy-loc-exceptions is not supported"));
    Ok(())
}

#[test]
fn validator_cli_accepts_files_at_exact_loc_target() -> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_fixture_with_line_count(250, None)?;
    let base = git_head(repo.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path())
        .output()?;
    assert!(
        output.status.success(),
        "validator should accept a governed file at exactly 250 lines\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_ignores_deleted_touched_file_and_keeps_other_violations() -> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_deleted_file_fixture(true)?;
    let base = git_head(repo.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path())
        .output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("src/too_large.rs has 251 lines"));
    assert!(!stderr.contains("reading touched file src/deleted.rs"));
    Ok(())
}

#[test]
fn validator_cli_accepts_a_committed_deleted_touched_file() -> Result<(), Box<dyn std::error::Error>> {
    let repo = touched_loc_deleted_file_fixture(false)?;
    let base = git_head(repo.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", base.trim()])
        .current_dir(repo.path())
        .output()?;

    assert!(
        output.status.success(),
        "deleted touched file must not produce a read failure\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn touched_loc_fixture(
    with_exception: bool,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let exception_text = with_exception
        .then_some("src/too_large.rs integration harness needs dedicated follow-up split\n");
    touched_loc_fixture_with_line_count(251, exception_text)
}

fn touched_loc_fixture_with_line_count(
    line_count: usize,
    exception_text: Option<&str>,
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
    let oversized = (0..line_count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect::<String>();
    std::fs::write(repo.join("src/too_large.rs"), oversized)?;
    if let Some(text) = exception_text {
        std::fs::write(repo.join(".codexy-loc-exceptions"), text)?;
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

fn touched_loc_deleted_file_fixture(
    add_remaining_violation: bool,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = temp.path();
    Command::new("git").args(["init", "-q"]).current_dir(repo).status()?;
    Command::new("git")
        .args(["config", "user.email", "codexy@example.test"])
        .current_dir(repo)
        .status()?;
    Command::new("git")
        .args(["config", "user.name", "Codexy Test"])
        .current_dir(repo)
        .status()?;
    std::fs::create_dir_all(repo.join("src"))?;
    std::fs::write(repo.join("src/deleted.rs"), "fn retained() {}\n")?;
    Command::new("git").args(["add", "."]).current_dir(repo).status()?;
    Command::new("git")
        .args(["commit", "-qm", "seed deleted file"])
        .current_dir(repo)
        .status()?;
    std::fs::remove_file(repo.join("src/deleted.rs"))?;
    if add_remaining_violation {
        let oversized = (0..251)
            .map(|index| format!("fn line_{index}() {{}}\n"))
            .collect::<String>();
        std::fs::write(repo.join("src/too_large.rs"), oversized)?;
    }
    Command::new("git").args(["add", "-A"]).current_dir(repo).status()?;
    Command::new("git")
        .args(["commit", "-qm", "delete touched file"])
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
