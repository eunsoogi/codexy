use crate::support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_does_not_credit_cfg_disabled_outline_module() -> TestResult {
    let repo = fixture("src/lib.rs", regular_lines(252))?;
    amend(repo.path())?;
    write(
        repo.path(),
        "src/lib.rs",
        &format!("#[cfg(any())]\nmod foo;\n{}", regular_lines(248)),
    )?;
    write(repo.path(), "src/foo.rs", &regular_lines_from(248, 4))?;

    assert_compiles_and_fails_closed(&repo)
}

#[test]
fn touched_loc_does_not_credit_cfg_disabled_inline_module() -> TestResult {
    let repo = fixture("src/lib.rs", regular_lines(252))?;
    amend(repo.path())?;
    write(
        repo.path(),
        "src/lib.rs",
        &format!(
            "#[cfg(any())]\nmod outer {{ mod helper; }}\n{}",
            regular_lines(248)
        ),
    )?;
    write(
        repo.path(),
        "src/outer/helper.rs",
        &regular_lines_from(248, 4),
    )?;

    assert_compiles_and_fails_closed(&repo)
}

fn assert_compiles_and_fails_closed(repo: &tempfile::TempDir) -> TestResult {
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(!output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn amend(root: &Path) -> TestResult {
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
