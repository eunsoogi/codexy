mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_accepts_trailing_trivia_on_default_declarations() -> TestResult {
    for suffix in ["// generated", "/* generated */"] {
        let repo = module_fixture(&format!("mod outer; {suffix}\n"))?;
        assert_rustc_and_validator_accept(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_accepts_trailing_trivia_on_same_line_path_declarations() -> TestResult {
    for suffix in ["// generated", "/* generated */"] {
        let repo = module_fixture(&format!(
            "#[path = \"generated/bar.rs\"] mod bar; {suffix}\n"
        ))?;
        assert_rustc_and_validator_accept(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_preserves_plain_declaration_control() -> TestResult {
    let repo = module_fixture("pub(crate) mod outer;\n")?;
    assert_rustc_and_validator_accept(&repo)
}

#[test]
fn touched_loc_does_not_parse_declarations_inside_strings() -> TestResult {
    let repo = module_fixture("const TEXT: &str = \"mod outer; // generated\";\n")?;
    assert_rustc_accepts_validator_rejects(&repo)
}

#[test]
fn touched_loc_does_not_parse_declarations_inside_comments() -> TestResult {
    for source in [
        "// mod outer; // generated\n",
        "/* mod outer; /* generated */ */\n",
    ] {
        let repo = module_fixture(source)?;
        assert_rustc_accepts_validator_rejects(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_resets_path_after_prior_declaration() -> TestResult {
    let repo = module_fixture("#[path = \"unused.rs\"] mod skipped;\nmod outer; // generated\n")?;
    assert_rustc_and_validator_accept(&repo)
}

#[test]
fn touched_loc_accepts_rust_trivia_between_mod_and_identifier() -> TestResult {
    for declaration in [
        "mod\touter;\n",
        "mod /* generated */ outer;\n",
        "pub(crate)\tmod outer;\n",
        "mod r#outer;\n",
    ] {
        let repo = module_fixture(declaration)?;
        assert_rustc_and_validator_accept(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_rejects_unclosed_comment_between_mod_and_identifier() -> TestResult {
    let repo = module_fixture("mod /* generated outer;\n")?;
    let rustc = compile(repo.path())?;
    assert!(!rustc.status.success());
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn module_fixture(source: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/generated/bar.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", source)?;
    write(
        repo.path(),
        "src/outer.rs",
        "#[path = \"generated/bar.rs\"]\nmod bar;\n",
    )?;
    write(repo.path(), "src/unused.rs", "")?;
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

fn assert_rustc_accepts_validator_rejects(repo: &tempfile::TempDir) -> TestResult {
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
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
