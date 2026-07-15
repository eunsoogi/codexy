mod support;

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_resolves_children_beside_path_attributed_module_file() -> TestResult {
    let repo = path_attributed_fixture("src/generated/helper.rs")?;
    let rustc = run(
        repo.path(),
        "rustc",
        &[
            "--crate-name",
            "fixture",
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "--out-dir",
            "target",
            "src/lib.rs",
        ],
    )?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_stem_child_for_path_attributed_module_file() -> TestResult {
    let repo = path_attributed_fixture("src/generated/bar/helper.rs")?;
    let rustc = run(
        repo.path(),
        "rustc",
        &[
            "--crate-name",
            "fixture",
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "--out-dir",
            "target",
            "src/lib.rs",
        ],
    )?;
    assert!(!rustc.status.success());
    assert!(stderr(&rustc).contains("file not found for module `helper`"));

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn cargo_metadata_discovers_workspace_target_outside_package() -> TestResult {
    let repo = workspace_target_fixture("../../shared/src/tool.rs", "shared/src/helper.rs")?;
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
    let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
    assert!(metadata["packages"].as_array().is_some_and(|packages| {
        packages.iter().any(|package| {
            package["targets"].as_array().is_some_and(|targets| {
                targets.iter().any(|target| {
                    target["src_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("/shared/src/tool.rs"))
                })
            })
        })
    }));

    let cargo = run(repo.path(), "cargo", &["check", "--offline", "--workspace"])?;
    assert!(cargo.status.success(), "cargo stderr:\n{}", stderr(&cargo));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_stem_child_for_out_of_package_target() -> TestResult {
    let repo = workspace_target_fixture("../../shared/src/tool.rs", "shared/src/tool/helper.rs")?;
    let cargo = run(repo.path(), "cargo", &["check", "--offline", "--workspace"])?;
    assert!(!cargo.status.success());
    assert!(stderr(&cargo).contains("file not found for module `helper`"));

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_rejects_escaping_or_non_equivalent_target_paths() -> TestResult {
    for target in ["../../../shared/src/tool.rs", "../shared/src/tool.rs"] {
        let repo = workspace_target_fixture(target, "shared/src/helper.rs")?;
        let output = validate(repo.path())?;
        assert!(
            !output.status.success(),
            "target {target} must not classify shared/src/tool.rs as a crate root"
        );
        assert!(stderr(&output).contains("multiline collapse"));
    }
    Ok(())
}

fn path_attributed_fixture(extracted_path: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/generated/bar.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/lib.rs",
        "#[path = \"generated/bar.rs\"]\nmod foo;\n",
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/generated/bar.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(repo.path(), extracted_path, &regular_lines_from(249, 3))?;
    Ok(repo)
}

fn workspace_target_fixture(target: &str, extracted_path: &str) -> TestResult<tempfile::TempDir> {
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
