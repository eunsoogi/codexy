mod support;

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_rejects_target_from_excluded_workspace_package() -> TestResult {
    let repo = excluded_package_fixture()?;
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

#[test]
fn touched_loc_rejects_target_outside_nested_repository() -> TestResult {
    let (outer, repo) = nested_repository_fixture(false)?;
    let metadata = run(
        &repo,
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
    assert!(
        metadata["packages"].as_array().is_some_and(|packages| {
            packages.iter().any(|package| package["name"] == "external")
        })
    );
    assert!(!outer.path().join("src/tool.rs").starts_with(&repo));

    let output = validate(&repo)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_rejects_target_from_manifest_outside_repository() -> TestResult {
    let (_outer, repo) = nested_repository_fixture(true)?;
    let metadata = run(
        &repo,
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
    let external = metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|package| package["name"] == "external")
        })
        .ok_or("external metadata package missing")?;
    assert!(
        external["manifest_path"]
            .as_str()
            .is_some_and(|path| !Path::new(path).starts_with(&repo))
    );
    let canonical_repo = repo.canonicalize()?;
    assert!(external["targets"].as_array().is_some_and(|targets| {
        targets.iter().any(|target| {
            target["src_path"]
                .as_str()
                .is_some_and(|path| Path::new(path).starts_with(&canonical_repo))
        })
    }));

    let output = validate(&repo)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn excluded_package_fixture() -> TestResult<tempfile::TempDir> {
    let repo = fixture("shared/src/tool.rs", regular_lines(252))?;
    write(
        repo.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/app\"]\nexclude = [\"fixtures/excluded\"]\nresolver = \"2\"\n",
    )?;
    write(
        repo.path(),
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    write(repo.path(), "crates/app/src/lib.rs", "")?;
    write(
        repo.path(),
        "fixtures/excluded/Cargo.toml",
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

fn nested_repository_fixture(
    target_inside_repo: bool,
) -> TestResult<(tempfile::TempDir, std::path::PathBuf)> {
    let outer = tempfile::tempdir()?;
    let repo = outer.path().join("repo");
    std::fs::create_dir(&repo)?;
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "codexy@example.test"][..],
        &["config", "user.name", "Codexy Test"][..],
    ] {
        let output = run(&repo, "git", args)?;
        assert!(output.status.success(), "git stderr:\n{}", stderr(&output));
    }
    write(&repo, "src/tool.rs", &regular_lines(252))?;
    let add = run(&repo, "git", &["add", "."])?;
    assert!(add.status.success(), "git add stderr:\n{}", stderr(&add));
    let commit = run(&repo, "git", &["commit", "-qm", "initial"])?;
    assert!(
        commit.status.success(),
        "git commit stderr:\n{}",
        stderr(&commit)
    );
    write(
        outer.path(),
        "Cargo.toml",
        "[workspace]\nmembers = [\"external\"]\nresolver = \"2\"\n",
    )?;
    let external_target = if target_inside_repo {
        repo.join("src/tool.rs")
            .canonicalize()?
            .to_string_lossy()
            .into_owned()
    } else {
        "../src/tool.rs".to_owned()
    };
    write(
        outer.path(),
        "external/Cargo.toml",
        &format!(
            "[package]\nname = \"external\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"tool\"\npath = \"{}\"\n",
            external_target
        ),
    )?;
    write(outer.path(), "src/tool.rs", "fn main() {}\n")?;
    write(
        &repo,
        "src/tool.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(&repo, "src/helper.rs", &regular_lines_from(249, 3))?;
    Ok((outer, repo))
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
