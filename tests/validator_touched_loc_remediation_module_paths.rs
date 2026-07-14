mod support;

use std::process::Command;

use serde_json::Value;
use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn cargo_metadata_discovers_directory_target_main_roots() -> TestResult {
    let repo = fixture("examples/foo/main.rs", String::new())?;
    for path in ["tests/foo/main.rs", "benches/foo/main.rs"] {
        write(repo.path(), path, "")?;
    }
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(repo.path())
        .output()?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let metadata: Value = serde_json::from_slice(&output.stdout)?;
    for (kind, path) in [
        ("example", "examples/foo/main.rs"),
        ("test", "tests/foo/main.rs"),
        ("bench", "benches/foo/main.rs"),
    ] {
        assert!(
            metadata["packages"][0]["targets"]
                .as_array()
                .is_some_and(|targets| {
                    targets.iter().any(|target| {
                        target["kind"]
                            .as_array()
                            .is_some_and(|kinds| kinds.iter().any(|candidate| candidate == kind))
                            && target["src_path"]
                                .as_str()
                                .is_some_and(|source| source.ends_with(path))
                    })
                })
        );
    }
    Ok(())
}

#[test]
fn touched_loc_honors_attributed_module_path() -> TestResult {
    let repo = attributed_module_fixture("src/helper.rs", "#[path = \"helper.rs\"]\n")?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_honors_path_through_stacked_outer_attributes() -> TestResult {
    let repo =
        attributed_module_fixture("src/helper.rs", "#[path = \"helper.rs\"]\n#[cfg(unix)]\n")?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_does_not_credit_default_path_for_attributed_module() -> TestResult {
    let repo = attributed_module_fixture("src/foo/helper.rs", "#[path = \"helper.rs\"]\n")?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_honors_path_through_blank_line_trivia() -> TestResult {
    let repo = attributed_module_fixture("src/helper.rs", "#[path = \"helper.rs\"]\n\n")?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_honors_path_through_line_comment_trivia() -> TestResult {
    let repo = attributed_module_fixture(
        "src/helper.rs",
        "#[path = \"helper.rs\"]\n// helper stays in the explicit path\n",
    )?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_honors_path_through_block_comment_trivia() -> TestResult {
    let repo = attributed_module_fixture(
        "src/helper.rs",
        "#[path = \"helper.rs\"]\n/* helper stays\n * in the explicit path\n */\n",
    )?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_clears_path_after_intervening_item() -> TestResult {
    let repo = attributed_module_fixture(
        "src/helper.rs",
        "#[path = \"helper.rs\"]\nconst INTERVENING: () = ();\n",
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn attributed_module_fixture(extracted_path: &str, prefix: &str) -> TestResult<tempfile::TempDir> {
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
        extracted_path,
        &regular_lines_from(retained_lines, 252 - retained_lines),
    )?;
    Ok(repo)
}
