use crate::support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_accepts_valid_restricted_module_visibility() -> TestResult {
    for visibility in [
        "pub ",
        "pub(crate) ",
        "pub(super) ",
        "pub(self) ",
        "pub(in crate::outer) ",
    ] {
        let repo = visibility_fixture(&format!("{visibility}mod helper;\n"))?;
        let rustc = compile(repo.path())?;
        assert!(
            rustc.status.success(),
            "visibility {visibility:?}\nrustc stderr:\n{}",
            stderr(&rustc)
        );
        let output = validate(repo.path())?;
        assert!(
            output.status.success(),
            "visibility {visibility:?}\nvalidator stderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn touched_loc_rejects_malformed_restricted_module_visibility() -> TestResult {
    for declaration in [
        "pub(foo) mod helper;\n",
        "pub(in) mod helper;\n",
        "pub(in crate::outer mod helper;\n",
    ] {
        let repo = visibility_fixture(declaration)?;
        let rustc = compile(repo.path())?;
        assert!(!rustc.status.success(), "declaration {declaration:?}");
        let output = validate(repo.path())?;
        assert!(!output.status.success(), "declaration {declaration:?}");
        assert!(stderr(&output).contains("multiline collapse"));
    }
    Ok(())
}

fn visibility_fixture(declaration: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/outer.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod outer;\n")?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/outer.rs",
        &format!("{declaration}{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/outer/helper.rs",
        &regular_lines_from(249, 3),
    )?;
    Ok(repo)
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
