use crate::support;

use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn cargo_metadata_discovers_workspace_custom_target_without_lockfile() -> TestResult {
    let repo = fixture(
        "shared/src/tool.rs",
        format!("{}fn main() {{}}\n", regular_lines(251)),
    )?;
    write(
        repo.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/app\"]\nresolver = \"2\"\n",
    )?;
    write(
        repo.path(),
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nserde = \"1\"\n\n[[bin]]\nname = \"tool\"\npath = \"../../shared/src/tool.rs\"\n",
    )?;
    amend(repo.path())?;
    write(
        repo.path(),
        "shared/src/tool.rs",
        &format!("mod helper;\n{}fn main() {{}}\n", regular_lines(248)),
    )?;
    write(
        repo.path(),
        "shared/src/helper.rs",
        &regular_lines_from(248, 3),
    )?;

    let cargo = run(repo.path(), "cargo", &["check", "--offline"])?;
    assert!(cargo.status.success(), "cargo stderr:\n{}", stderr(&cargo));
    let lockfile = repo.path().join("Cargo.lock");
    std::fs::remove_file(&lockfile)?;
    assert!(!lockfile.exists());
    let metadata = run(
        repo.path(),
        "cargo",
        &[
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ],
    )?;
    assert!(
        metadata.status.success(),
        "cargo metadata stderr:\n{}",
        stderr(&metadata)
    );
    assert!(!lockfile.exists());
    assert!(
        !include_str!("../src/validation/touched_loc_remediation/rust_module/origin.rs")
            .contains("\"--locked\"")
    );

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn amend(root: &std::path::Path) -> TestResult {
    let add = run(root, "git", &["add", "."])?;
    assert!(add.status.success(), "git add stderr:\n{}", stderr(&add));
    let commit = run(root, "git", &["commit", "--amend", "--no-edit", "-q"])?;
    assert!(commit.status.success(), "git stderr:\n{}", stderr(&commit));
    Ok(())
}

fn run(root: &std::path::Path, program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).current_dir(root).output()
}
