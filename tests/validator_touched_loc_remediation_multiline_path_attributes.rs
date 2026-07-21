use crate::support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_accepts_multiline_path_attributes() -> TestResult {
    for source in [
        "#[path =\n    \"generated/bar.rs\"]\nmod bar;\n",
        "#[path\n    =\n    r#\"generated/bar.rs\"#\n] mod bar; // generated\n",
    ] {
        let repo = module_fixture(source)?;
        assert_rustc_and_validator_accept(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_accepts_path_attributes_with_block_comment_trivia() -> TestResult {
    for source in [
        "#[path/* generated */ = \"generated/bar.rs\"]\nmod bar;\n",
        "#[path/* generated /* nested */ comment */ = \"generated/bar.rs\"]\nmod bar;\n",
    ] {
        let repo = module_fixture(source)?;
        assert_rustc_and_validator_accept(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_rejects_malformed_direct_path_comment_trivia() -> TestResult {
    for source in [
        "#[path/* generated = \"generated/bar.rs\"]\nmod bar;\n",
        "#[path/not_a_comment = \"generated/bar.rs\"]\nmod bar;\n",
    ] {
        let repo = module_fixture(source)?;
        assert_rustc_and_validator_reject(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_resets_multiline_path_after_prior_declaration() -> TestResult {
    let repo = nested_module_fixture("#[path =\n    \"unused.rs\"] mod skipped;\nmod outer;\n")?;
    assert_rustc_and_validator_accept(&repo)
}

#[test]
fn touched_loc_rejects_malformed_multiline_path_attributes() -> TestResult {
    for source in [
        "#[path =\n    \"generated/bar.rs\" trailing]\nmod bar;\n",
        "#[path =\n    \"generated/bar.rs\"\nmod bar;\n",
    ] {
        let repo = module_fixture(source)?;
        assert_rustc_and_validator_reject(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_clears_multiline_path_before_intervening_item() -> TestResult {
    let repo = module_fixture(
        "#[path =\n    \"generated/bar.rs\"]\nconst INTERVENING: () = ();\nmod bar;\n",
    )?;
    assert_rustc_and_validator_reject(&repo)
}

fn module_fixture(source: &str) -> TestResult<tempfile::TempDir> {
    fixture_with_sources(source, None)
}

fn nested_module_fixture(source: &str) -> TestResult<tempfile::TempDir> {
    fixture_with_sources(source, Some("#[path = \"generated/bar.rs\"]\nmod bar;\n"))
}

fn fixture_with_sources(
    root_source: &str,
    outer_source: Option<&str>,
) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/generated/bar.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", root_source)?;
    write(repo.path(), "src/unused.rs", "")?;
    if let Some(source) = outer_source {
        write(repo.path(), "src/outer.rs", source)?;
    }
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/generated/bar.rs",
        &format!("mod extracted;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/generated/extracted.rs",
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

fn assert_rustc_and_validator_reject(repo: &tempfile::TempDir) -> TestResult {
    let rustc = compile(repo.path())?;
    assert!(!rustc.status.success());
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
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
