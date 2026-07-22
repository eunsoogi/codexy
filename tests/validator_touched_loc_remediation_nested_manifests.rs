use crate::support;

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn cargo_metadata_discovers_independent_nested_target_below_root_manifest() -> TestResult {
    let repo = nested_package_below_root_fixture()?;
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

#[test]
fn cargo_metadata_discovers_nested_package_target_without_root_manifest() -> TestResult {
    let repo = nested_package_target_fixture("../../shared/src/tool.rs", "shared/src/helper.rs")?;
    let metadata = run(
        repo.path(),
        "cargo",
        &[
            "metadata",
            "--manifest-path",
            "crates/app/Cargo.toml",
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
    let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
    assert!(metadata["packages"].as_array().is_some_and(|packages| {
        packages.iter().any(|package| {
            package["targets"].as_array().is_some_and(|targets| {
                targets.iter().any(|target| {
                    target["src_path"]
                        .as_str()
                        .is_some_and(|path| {
                            Path::new(path).ends_with(Path::new("shared/src/tool.rs"))
                        })
                })
            })
        })
    }));

    let cargo = run(
        repo.path(),
        "cargo",
        &[
            "check",
            "--manifest-path",
            "crates/app/Cargo.toml",
            "--offline",
        ],
    )?;
    assert!(cargo.status.success(), "cargo stderr:\n{}", stderr(&cargo));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_target_from_excluded_nested_workspace_package() -> TestResult {
    let repo = excluded_nested_package_fixture()?;
    let metadata = run(
        repo.path(),
        "cargo",
        &[
            "metadata",
            "--manifest-path",
            "workspace/Cargo.toml",
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
    let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
    assert!(
        metadata["packages"].as_array().is_some_and(|packages| {
            packages.iter().all(|package| package["name"] != "excluded")
        })
    );

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn nested_package_target_fixture(
    target: &str,
    extracted_path: &str,
) -> TestResult<tempfile::TempDir> {
    let repo = fixture(
        "shared/src/tool.rs",
        format!("{}fn main() {{}}\n", regular_lines(251)),
    )?;
    write(
        repo.path(),
        "crates/app/Cargo.toml",
        &format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"tool\"\npath = \"{target}\"\n"
        ),
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "shared/src/tool.rs",
        &format!("mod helper;\n{}fn main() {{}}\n", regular_lines(248)),
    )?;
    write(repo.path(), extracted_path, &regular_lines_from(248, 3))?;
    Ok(repo)
}

fn nested_package_below_root_fixture() -> TestResult<tempfile::TempDir> {
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
        "tools/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n\n[[bin]]\nname = \"tool\"\npath = \"../../shared/src/tool.rs\"\n",
    )?;
    amend_fixture(repo.path())?;
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
    Ok(repo)
}

fn excluded_nested_package_fixture() -> TestResult<tempfile::TempDir> {
    let repo = fixture("shared/src/tool.rs", regular_lines(252))?;
    write(
        repo.path(),
        "workspace/Cargo.toml",
        "[workspace]\nmembers = [\"app\"]\nexclude = [\"excluded\"]\nresolver = \"2\"\n",
    )?;
    write(
        repo.path(),
        "workspace/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    write(repo.path(), "workspace/app/src/lib.rs", "")?;
    write(
        repo.path(),
        "workspace/excluded/Cargo.toml",
        "[package]\nname = \"excluded\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"tool\"\npath = \"../../shared/src/tool.rs\"\n",
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "shared/src/tool.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "shared/src/helper.rs",
        &regular_lines_from(249, 3),
    )?;
    Ok(repo)
}

fn amend_fixture(root: &Path) -> TestResult {
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

fn run(root: &Path, program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).current_dir(root).output()
}
