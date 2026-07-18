use crate::support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_does_not_credit_default_path_after_enabled_cfg_attr_path() -> TestResult {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    amend(repo.path())?;
    write(
        repo.path(),
        "src/foo.rs",
        &format!(
            "#[cfg_attr(unix, path = \"generated.rs\")]\nmod helper;\n{}",
            regular_lines(248)
        ),
    )?;
    write(
        repo.path(),
        "src/generated.rs",
        "pub const ACTUAL: () = ();\n",
    )?;
    write(
        repo.path(),
        "src/foo/helper.rs",
        &regular_lines_from(248, 4),
    )?;

    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_does_not_credit_default_path_after_cfg_attr_path_comment_trivia() -> TestResult {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    amend(repo.path())?;
    write(
        repo.path(),
        "src/foo.rs",
        &format!(
            "#[cfg_attr(unix, path/* generated */ = \"generated.rs\")]\nmod helper;\n{}",
            regular_lines(248)
        ),
    )?;
    write(
        repo.path(),
        "src/generated.rs",
        "pub const ACTUAL: () = ();\n",
    )?;
    write(
        repo.path(),
        "src/foo/helper.rs",
        &regular_lines_from(248, 4),
    )?;

    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_rejects_unclosed_cfg_attr_path_comment() -> TestResult {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    amend(repo.path())?;
    write(
        repo.path(),
        "src/foo.rs",
        &format!(
            "#[cfg_attr(unix, path/* generated = \"generated.rs\")]\nmod helper;\n{}",
            regular_lines(248)
        ),
    )?;
    write(
        repo.path(),
        "src/foo/helper.rs",
        &regular_lines_from(248, 4),
    )?;

    let rustc = compile(repo.path())?;
    assert!(!rustc.status.success());
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_preserves_balanced_non_path_cfg_attr_and_path_controls() -> TestResult {
    for (attribute, extracted_path) in [
        (
            "#[cfg_attr(all(unix, path = \"x\"), allow(dead_code))]\n",
            "src/foo/helper.rs",
        ),
        (
            "#[cfg_attr(any(), path = \"generated.rs\")]\n",
            "src/foo/helper.rs",
        ),
        ("#[cfg_attr(unix, allow(dead_code))]\n", "src/foo/helper.rs"),
        ("#[path = \"generated.rs\"]\n", "src/generated.rs"),
    ] {
        let repo = fixture("src/foo.rs", regular_lines(252))?;
        write(repo.path(), "src/lib.rs", "mod foo;\n")?;
        amend(repo.path())?;
        write(
            repo.path(),
            "src/foo.rs",
            &format!("{attribute}mod helper;\n{}", regular_lines(248)),
        )?;
        write(repo.path(), extracted_path, &regular_lines_from(248, 4))?;

        let rustc = compile(repo.path())?;
        assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
        let output = validate(repo.path())?;
        assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn touched_loc_honors_path_attributes_with_token_whitespace() -> TestResult {
    for attribute in [
        "# [path = \"generated.rs\"]\n",
        "#[ path = \"generated.rs\"]\n",
    ] {
        let repo = fixture("src/foo.rs", regular_lines(250))?;
        write(repo.path(), "src/lib.rs", "mod foo;\n")?;
        amend(repo.path())?;
        write(
            repo.path(),
            "src/foo.rs",
            &format!("{attribute}mod helper;\n{}", regular_lines(248)),
        )?;
        write(repo.path(), "src/generated.rs", &regular_lines_from(248, 2))?;

        let rustc = compile(repo.path())?;
        assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
        let output = validate(repo.path())?;
        assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn touched_loc_preserves_cfg_attr_with_comment_delimiters() -> TestResult {
    for attribute in [
        "#[cfg_attr(all(unix /* ( */, windows), allow(dead_code))]",
        "#[cfg_attr(all(\n        unix, // (\n        windows\n    ), allow(dead_code))]",
    ] {
        let retained = 250 - attribute.lines().count() - 3;
        let repo = fixture("src/foo/outer/helper.rs", regular_lines(250))?;
        write(repo.path(), "src/lib.rs", "mod foo;\n")?;
        amend(repo.path())?;
        write(
            repo.path(),
            "src/foo.rs",
            &format!(
                "mod outer {{\n    {attribute}\n    mod helper;\n}}\n{}",
                regular_lines(retained)
            ),
        )?;
        write(
            repo.path(),
            "src/foo/outer/helper.rs",
            &regular_lines_from(retained, 250 - retained),
        )?;

        let rustc = compile(repo.path())?;
        assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
        let output = validate(repo.path())?;
        assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn touched_loc_discovers_default_module_inside_inline_scope() -> TestResult {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    amend(repo.path())?;
    write(
        repo.path(),
        "src/foo.rs",
        &format!("mod outer {{ mod helper; }}\n{}", regular_lines(249)),
    )?;
    write(
        repo.path(),
        "src/foo/outer/helper.rs",
        &regular_lines_from(249, 3),
    )?;

    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
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
