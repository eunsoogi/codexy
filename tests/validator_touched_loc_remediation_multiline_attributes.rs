use crate::support;

use std::path::Path;
use std::process::{Command, Output};

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_preserves_path_through_multiline_outer_attribute() -> TestResult {
    let repo = attributed_module_fixture(
        "#[path = \"helper.rs\"]\n#[cfg(any(\n    unix,\n    windows,\n))]\n",
    )?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_parses_module_on_multiline_attribute_closing_line() -> TestResult {
    let repo = attributed_module_fixture(
        "#[path = \"helper.rs\"]\n#[cfg(any(\n    unix,\n    windows,\n))] ",
    )?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_accepts_multiline_non_path_cfg_attr_before_default_module() -> TestResult {
    let repo = default_module_fixture("#[cfg_attr(\n    unix,\n    allow(dead_code)\n)]\n")?;
    let rustc = compile_library(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_multiline_cfg_attr_path_before_default_module() -> TestResult {
    let repo = default_module_fixture("#[cfg_attr(\n    unix,\n    path = \"generated.rs\"\n)]\n")?;
    write(
        repo.path(),
        "src/generated.rs",
        "pub const ACTUAL: () = ();\n",
    )?;
    let rustc = compile_library(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_clears_path_after_item_following_multiline_attribute() -> TestResult {
    let repo = attributed_module_fixture(
        "#[path = \"helper.rs\"]\n#[cfg(any(\n    unix,\n    windows,\n))]\nconst INTERVENING: () = ();\n",
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_clears_path_for_item_on_multiline_attribute_closing_line() -> TestResult {
    let repo = attributed_module_fixture(
        "#[path = \"helper.rs\"]\n#[cfg(any(\n    unix,\n    windows,\n))] const INTERVENING: () = ();\n",
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_rejects_malformed_path_before_multiline_attribute() -> TestResult {
    let repo = attributed_module_fixture(
        "#[path = \"helper.rs\"] trailing tokens\n#[cfg(any(\n    unix,\n    windows,\n))]\n",
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_honors_path_through_same_line_stacked_outer_attributes() -> TestResult {
    let repo = attributed_module_fixture("#[path = \"helper.rs\"] #[cfg(unix)] ")?;
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_clears_path_at_same_line_attribute_item_boundary() -> TestResult {
    let repo = attributed_module_fixture(
        "#[path = \"helper.rs\"] #[cfg(unix)] const INTERVENING: () = ();\n",
    )?;
    let rustc = compile(repo.path())?;
    assert!(rustc.status.success(), "rustc stderr:\n{}", stderr(&rustc));
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn attributed_module_fixture(prefix: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    let declaration = format!("{prefix}mod helper;\n");
    let retained_lines = 250 - declaration.lines().count();
    write(
        repo.path(),
        "src/foo.rs",
        &format!("{declaration}{}", regular_lines(retained_lines)),
    )?;
    write(
        repo.path(),
        "src/helper.rs",
        &regular_lines_from(retained_lines, 252 - retained_lines),
    )?;
    Ok(repo)
}

fn default_module_fixture(prefix: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    write(repo.path(), "src/lib.rs", "mod foo;\n")?;
    amend_fixture(repo.path())?;
    let declaration = format!("{prefix}mod helper;\n");
    let retained_lines = 250 - declaration.lines().count();
    write(
        repo.path(),
        "src/foo.rs",
        &format!("{declaration}{}", regular_lines(retained_lines)),
    )?;
    write(
        repo.path(),
        "src/foo/helper.rs",
        &regular_lines_from(retained_lines, 252 - retained_lines),
    )?;
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
    Command::new("rustc")
        .args(["--crate-type", "lib", "src/foo.rs"])
        .current_dir(root)
        .output()
}

fn compile_library(root: &Path) -> std::io::Result<Output> {
    Command::new("rustc")
        .args(["--crate-type", "lib", "src/lib.rs"])
        .current_dir(root)
        .output()
}

fn run(root: &Path, program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).current_dir(root).output()
}
