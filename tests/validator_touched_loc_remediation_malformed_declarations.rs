mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_rejects_default_fallback_after_malformed_path() -> TestResult {
    let repo = outer_fixture(
        "#[path = \"bad\\u{zz}\"]\nmod helper;\n",
        "src/foo/helper.rs",
    )?;
    assert_rustc_rejects_and_validator_fails_closed(&repo)
}

#[test]
fn touched_loc_preserves_valid_default_and_path_controls() -> TestResult {
    for (declaration, extracted_path) in [
        ("mod helper;\n", "src/foo/helper.rs"),
        (
            "#[path = \"generated/helper.rs\"]\nmod helper;\n",
            "src/generated/helper.rs",
        ),
    ] {
        let repo = outer_fixture(declaration, extracted_path)?;
        assert_rustc_and_validator_accept(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_rejects_inline_origin_after_invalid_visibility() -> TestResult {
    let repo = inline_fixture("pub(foo) ")?;
    assert_rustc_rejects_and_validator_fails_closed(&repo)
}

#[test]
fn touched_loc_preserves_valid_restricted_inline_visibility() -> TestResult {
    for visibility in [
        "",
        "pub(crate) ",
        "pub(super) ",
        "pub(self) ",
        "pub(in crate::outer) ",
    ] {
        let repo = inline_fixture(visibility)?;
        assert_rustc_and_validator_accept(&repo)?;
    }
    Ok(())
}

fn outer_fixture(declaration: &str, extracted_path: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    amend_fixture(repo.path())?;
    let retained = 250 - declaration.lines().count();
    write(
        repo.path(),
        "src/foo.rs",
        &format!("{declaration}{}", regular_lines(retained)),
    )?;
    write(
        repo.path(),
        extracted_path,
        &regular_lines_from(retained, 252 - retained),
    )?;
    Ok(repo)
}

fn inline_fixture(visibility: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/outer/thread/tls.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/lib.rs",
        &format!(
            "mod outer {{\n    {visibility}mod thread {{\n        #[path = \"tls.rs\"]\n        mod local_data;\n    }}\n}}\n"
        ),
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/outer/thread/tls.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/outer/thread/helper.rs",
        &regular_lines_from(249, 3),
    )?;
    Ok(repo)
}

fn assert_rustc_rejects_and_validator_fails_closed(repo: &tempfile::TempDir) -> TestResult {
    let rustc = compile(repo.path())?;
    assert!(!rustc.status.success());
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
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
