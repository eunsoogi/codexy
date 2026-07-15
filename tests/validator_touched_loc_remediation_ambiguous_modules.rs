mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FLAT_MODULE: &str = "src/too_large/helper.rs";
const DIRECTORY_MODULE: &str = "src/too_large/helper/mod.rs";

#[test]
fn touched_loc_rejects_ambiguous_default_module_candidates() -> TestResult {
    let repo = module_fixture(FLAT_MODULE)?;
    write(repo.path(), DIRECTORY_MODULE, extracted_source())?;

    let rustc = compile(repo.path())?;
    assert!(!rustc.status.success());
    assert!(stderr(&rustc).contains("found at both"));

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_accepts_flat_default_module_candidate() -> TestResult {
    let repo = module_fixture(FLAT_MODULE)?;
    write(repo.path(), FLAT_MODULE, &with_existing_item())?;
    assert_rustc_and_validator_accept(&repo)
}

#[test]
fn touched_loc_accepts_directory_default_module_candidate() -> TestResult {
    let repo = module_fixture(DIRECTORY_MODULE)?;
    write(repo.path(), DIRECTORY_MODULE, &with_existing_item())?;
    assert_rustc_and_validator_accept(&repo)
}

fn module_fixture(existing_candidate: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/too_large.rs", baseline_source())?;
    write(repo.path(), "src/lib.rs", "mod too_large;\n")?;
    write(repo.path(), existing_candidate, "fn existing() {}\n")?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    Ok(repo)
}

fn baseline_source() -> String {
    format!("mod helper;\n{}{}", regular_lines(249), extracted_source())
}

fn with_existing_item() -> String {
    format!("fn existing() {{}}\n{}", extracted_source())
}

fn extracted_source() -> &'static str {
    "fn extracted() {\n    let summary =\n        format!(\n            \"status\"\n        );\n    drop(summary);\n}\n"
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
    assert!(
        commit.status.success(),
        "git commit stderr:\n{}",
        stderr(&commit)
    );
    Ok(())
}

fn compile(root: &Path) -> std::io::Result<Output> {
    run(
        root,
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
    )
}

fn run(root: &Path, program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).current_dir(root).output()
}
