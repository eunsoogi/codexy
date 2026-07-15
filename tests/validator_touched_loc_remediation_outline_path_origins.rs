mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_resolves_outline_path_from_source_directory() -> TestResult {
    let repo = outline_module_fixture(
        "#[path = \"generated/bar.rs\"]\nmod bar;\n",
        "src/generated/bar.rs",
        "src/generated/helper.rs",
    )?;
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_preserves_default_child_scope_for_outline_module() -> TestResult {
    let repo = outline_module_fixture("mod bar;\n", "src/foo/bar.rs", "src/foo/bar/helper.rs")?;
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));

    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_ignores_cfg_disabled_path_alias_for_outline_origin() -> TestResult {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/lib.rs",
        "mod foo;\n#[cfg(any())]\n#[path = \"foo.rs\"]\nmod alias;\n",
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/foo.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/foo/helper.rs",
        &regular_lines_from(249, 3),
    )?;
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_does_not_credit_outline_path_against_stem_directory() -> TestResult {
    let repo = outline_module_fixture(
        "#[path = \"../generated/bar.rs\"]\nmod bar;\n",
        "src/generated/bar.rs",
        "src/generated/helper.rs",
    )?;
    let rustc = compile(repo.path())?;
    assert!(!rustc.status.success());
    assert!(stderr(&rustc).contains("couldn't read"));

    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn outline_module_fixture(
    declaration: &str,
    target: &str,
    extracted_path: &str,
) -> TestResult<tempfile::TempDir> {
    let repo = fixture(target, regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    write(repo.path(), "src/foo.rs", declaration)?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        target,
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(repo.path(), extracted_path, &regular_lines_from(249, 3))?;
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
