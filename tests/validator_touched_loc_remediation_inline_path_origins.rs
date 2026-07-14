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
    assert_inline_ancestor_path("")
}

#[test]
fn touched_loc_honors_path_on_restricted_visibility_inline_ancestor() -> TestResult {
    for visibility in ["pub(crate) ", "pub(in crate) "] {
        assert_inline_ancestor_path(visibility)?;
    }
    Ok(())
}

#[test]
fn touched_loc_clears_inline_path_after_completed_intervening_items() -> TestResult {
    for item in [
        "trait Marker {}\n",
        "struct Marker<const N: usize = { 1 }>;\n",
        "type Alias = ();\n",
        "static MARKER: () = ();\n",
    ] {
        let repo = intervening_item_fixture(item, "thread_files")?;
        let rustc = compile(repo.path())?;
        assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
        assert_validator_fails_closed(&repo)?;
    }
    Ok(())
}

#[test]
fn touched_loc_fails_closed_after_malformed_intervening_item_prefixes() -> TestResult {
    for item in [
        "trait Marker ",
        "type Alias = ",
        "static MARKER: ",
        "use foo::{bar} ",
        "type Alias = Array<{ 1 }> ",
        "struct Marker<const N: usize = { 1 }> ",
    ] {
        let repo = intervening_item_fixture(item, "thread")?;
        let rustc = compile(repo.path())?;
        assert!(!rustc.status.success());
        assert_validator_fails_closed(&repo)?;
    }
    Ok(())
}

fn assert_inline_ancestor_path(visibility: &str) -> TestResult {
    let repo = fixture("src/thread_files/tls.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/lib.rs",
        &format!(
            "#[path = \"thread_files\"]\n{visibility}mod thread {{\n    #[path = \"tls.rs\"]\n    mod local_data;\n}}\n"
        ),
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

fn intervening_item_fixture(item: &str, tracked_module: &str) -> TestResult<tempfile::TempDir> {
    let tracked_tls = format!("src/{tracked_module}/tls.rs");
    let tracked_helper = format!("src/{tracked_module}/helper.rs");
    let control_module = if tracked_module == "thread" {
        "thread_files"
    } else {
        "thread"
    };
    let repo = fixture(&tracked_tls, regular_lines(252))?;
    write(
        repo.path(),
        "src/lib.rs",
        &format!(
            "#[path = \"thread_files\"]\n{item}mod thread {{\n    #[path = \"tls.rs\"]\n    mod local_data;\n}}\n"
        ),
    )?;
    write(repo.path(), &format!("src/{control_module}/tls.rs"), "")?;
    amend_fixture(repo.path())?;
    write(
        repo.path(),
        &tracked_tls,
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(repo.path(), &tracked_helper, &regular_lines_from(249, 3))?;
    Ok(repo)
}

fn assert_validator_fails_closed(repo: &tempfile::TempDir) -> TestResult {
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
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
