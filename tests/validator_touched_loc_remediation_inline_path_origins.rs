mod support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_resolves_children_beside_nested_inline_path_module() -> TestResult {
    let repo = fixture("src/outer/bar.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/lib.rs",
        "mod outer {\n    #[path = \"bar.rs\"]\n    mod foo;\n}\n",
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/outer/bar.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/outer/helper.rs",
        &regular_lines_from(249, 3),
    )?;

    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_honors_path_on_inline_ancestor() -> TestResult {
    let repo = fixture("src/thread_files/tls.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/lib.rs",
        "#[path = \"thread_files\"]\nmod thread {\n    #[path = \"tls.rs\"]\n    mod local_data;\n}\n",
    )?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/thread_files/tls.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/thread_files/helper.rs",
        &regular_lines_from(249, 3),
    )?;

    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_preserves_lifetime_comment_and_string_controls() -> TestResult {
    let repo = fixture("src/outer/bar.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod outer { mod bar; }\n")?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        "src/outer/bar.rs",
        &format!(
            "fn marker<'a>() {{}}\n// ' mod forged {{ #[path = \"../../bar.rs\"] mod fake; }}\nconst TEXT: &str = \"#[path = \\\"../../bar.rs\\\"] mod fake;\";\nconst RAW: &str = r#\"#[path = \\\"../../bar.rs\\\"] mod fake;\"#;\nmod helper;\n{}",
            regular_lines(245)
        ),
    )?;
    write(repo.path(), "src/outer/bar/helper.rs", "")?;
    write(
        repo.path(),
        "src/outer/helper.rs",
        &regular_lines_from(245, 7),
    )?;

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
