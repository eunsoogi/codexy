use crate::support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FLAT_MODULE: &str = "src/foo.rs";
const DIRECTORY_MODULE: &str = "src/foo/mod.rs";

#[test]
fn touched_loc_rejects_path_origin_through_ambiguous_ancestor() -> TestResult {
    let repo = origin_fixture(
        FLAT_MODULE,
        "#[path = \"generated/bar.rs\"]\nmod bar;\n",
        Some((DIRECTORY_MODULE, "")),
    )?;

    let rustc = compile(repo.path())?;
    assert!(!rustc.status.success());
    assert!(stderr(&rustc).contains("found at both"));

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_accepts_path_origin_through_flat_ancestor() -> TestResult {
    let repo = origin_fixture(
        FLAT_MODULE,
        "#[path = \"generated/bar.rs\"]\nmod bar;\n",
        None,
    )?;
    assert_rustc_and_validator_accept(&repo)
}

#[test]
fn touched_loc_accepts_path_origin_through_directory_ancestor() -> TestResult {
    let repo = origin_fixture(
        DIRECTORY_MODULE,
        "#[path = \"../generated/bar.rs\"]\nmod bar;\n",
        None,
    )?;
    assert_rustc_and_validator_accept(&repo)
}

fn origin_fixture(
    module_path: &str,
    module_source: &str,
    other_module: Option<(&str, &str)>,
) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/generated/bar.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    write(repo.path(), module_path, module_source)?;
    if let Some((path, source)) = other_module {
        write(repo.path(), path, source)?;
    }
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/generated/bar.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/generated/helper.rs",
        &regular_lines_from(249, 3),
    )?;
    Ok(repo)
}

fn assert_rustc_and_validator_accept(repo: &tempfile::TempDir) -> TestResult {
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn amend_fixture(root: &Path) -> TestResult {
    let add = run(root, "git", &["add", "."])?;
    assert!(add.status.success(), "git add stderr:\n{}", stderr(&add));
    let commit = run(root, "git", &["commit", "--amend", "--no-edit", "-q"])?;
    assert!(commit.status.success(), "git stderr:\n{}", stderr(&commit));
    Ok(())
}

fn compile(root: &Path) -> std::io::Result<Output> {
    run(
        root,
        "rustc",
        &[
            "--edition=2024",
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
    )
}

fn run(root: &Path, program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).current_dir(root).output()
}
