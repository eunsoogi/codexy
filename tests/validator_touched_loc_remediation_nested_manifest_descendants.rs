mod support;

use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn cargo_metadata_discovers_independent_target_below_nested_package_manifest() -> TestResult {
    let repo = fixture(
        "shared/src/tool.rs",
        format!("{}fn main() {{}}\n", regular_lines(251)),
    )?;
    write(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\nmembers = []\nresolver = \"2\"\n",
    )?;
    write(repo.path(), "src/lib.rs", "")?;
    write(
        repo.path(),
        "tools/Cargo.toml",
        "[package]\nname = \"tools\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\nmembers = []\nresolver = \"2\"\n",
    )?;
    write(repo.path(), "tools/src/lib.rs", "")?;
    write(
        repo.path(),
        "tools/app/Cargo.toml",
        "[package]\nname = \"nested-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\nmembers = []\nresolver = \"2\"\n\n[[bin]]\nname = \"tool\"\npath = \"../../shared/src/tool.rs\"\n",
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

    let metadata = run(
        repo.path(),
        "cargo",
        &[
            "metadata",
            "--manifest-path",
            "tools/app/Cargo.toml",
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
    let cargo = run(
        repo.path(),
        "cargo",
        &[
            "check",
            "--manifest-path",
            "tools/app/Cargo.toml",
            "--offline",
        ],
    )?;
    assert!(cargo.status.success(), "cargo stderr:\n{}", stderr(&cargo));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn amend(root: &std::path::Path) -> TestResult {
    let add = run(root, "git", &["add", "."])?;
    assert!(add.status.success(), "git add stderr:\n{}", stderr(&add));
    let commit = run(root, "git", &["commit", "--amend", "--no-edit", "-q"])?;
    assert!(
        commit.status.success(),
        "git commit stderr:\n{}",
        stderr(&commit)
    );
    Ok(())
}

fn run(root: &std::path::Path, command: &str, args: &[&str]) -> TestResult<Output> {
    Ok(Command::new(command)
        .args(args)
        .current_dir(root)
        .output()?)
}
