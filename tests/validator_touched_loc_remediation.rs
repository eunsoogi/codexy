use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_rejects_blank_line_only_remediation() -> TestResult {
    let repo = fixture("src/too_large.rs", blank_line_source())?;
    std::fs::write(repo.path().join("src/too_large.rs"), regular_lines(250))?;

    let output = validate(repo.path())?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("blank-line deletion"));
    Ok(())
}

#[test]
fn touched_loc_rejects_multiline_collapse_remediation() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!("{}let summary = format!(\"status\");\n", regular_lines(249)),
    )?;

    let output = validate(repo.path())?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_rejects_mixed_token_collapse_over_eight_lines() -> TestResult {
    let repo = fixture("src/too_large.rs", mixed_token_collapse_source())?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!(
            "{}let report = format!(\"{{}}/{{}}\", \"status\", 1 + 2, true, \"ok\", 3 * 4, 5 - 1, 6 / 2,);\n",
            regular_lines(241)
        ),
    )?;

    let output = validate(repo.path())?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_rejects_collapse_hidden_by_unrelated_rename() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!(
            "{}let renamed_summary = format!(\"status\");\n",
            regular_lines(249)
        ),
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_collapse_with_independent_structural_remediation() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!(
            "mod helper;\n{}let summary = format!(\"status\");\n",
            regular_lines(248)
        ),
    )?;
    write(
        repo.path(),
        "src/too_large/helper.rs",
        "fn line_248() {}\nlet summary = format!(\n    \"status\"\n);\n",
    )?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_allows_structural_remediation_variants() -> TestResult {
    for (label, path, source, replacement, extracted) in [
        (
            "nested helper extraction from named module",
            "src/foo.rs",
            regular_lines(252),
            format!("mod helper;\n{}", regular_lines(249)),
            Some(("src/foo/helper.rs", regular_lines_from(249, 3))),
        ),
        (
            "nested module splitting from mod.rs",
            "src/foo/mod.rs",
            regular_lines(252),
            format!("mod extracted;\n{}", regular_lines(249)),
            Some(("src/foo/extracted.rs", regular_lines_from(249, 3))),
        ),
        (
            "binary crate root module splitting",
            "src/bin/too_large.rs",
            regular_lines(252),
            format!("mod worker;\n{}", regular_lines(249)),
            Some(("src/bin/worker.rs", regular_lines_from(249, 3))),
        ),
        (
            "test-target splitting",
            "tests/too_large.rs",
            regular_lines(252),
            format!("mod scenarios;\n{}", regular_lines(249)),
            Some(("tests/scenarios.rs", regular_lines_from(249, 3))),
        ),
        (
            "responsibility separation",
            "src/too_large.rs",
            regular_lines(252),
            format!("mod worker;\n{}", regular_lines(249)),
            Some(("src/too_large/worker.rs", regular_lines_from(249, 3))),
        ),
        (
            "real duplication removal",
            "src/too_large.rs",
            format!(
                "{}fn duplicate() {{}}\nfn duplicate() {{}}\nfn duplicate() {{}}\n",
                regular_lines(249)
            ),
            format!("{}fn duplicate() {{}}\n", regular_lines(249)),
            None,
        ),
    ] {
        let repo = fixture(path, source)?;
        write(repo.path(), path, &replacement)?;
        if let Some((extracted_path, extracted_text)) = extracted {
            write(repo.path(), extracted_path, &extracted_text)?;
        }

        let output = validate(repo.path())?;

        assert!(
            output.status.success(),
            "{label} should remain eligible\nstderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn touched_loc_rejects_sibling_files_for_nested_module_declarations() -> TestResult {
    for (path, sibling) in [
        ("src/foo.rs", "src/helper.rs"),
        ("src/foo/mod.rs", "src/foo.rs"),
    ] {
        let repo = fixture(path, multiline_source())?;
        write(repo.path(), sibling, "let summary = format!(\"status\");\n")?;
        write(
            repo.path(),
            path,
            &format!("mod helper;\n{}", regular_lines(249)),
        )?;

        let output = validate(repo.path())?;

        assert!(!output.status.success(), "{path} must ignore {sibling}");
        assert!(stderr(&output).contains("multiline collapse"));
    }
    Ok(())
}

fn fixture(path: &str, source: String) -> TestResult<tempfile::TempDir> {
    let repo = tempfile::tempdir()?;
    run(repo.path(), &["init", "-q"])?;
    run(
        repo.path(),
        &["config", "user.email", "codexy@example.test"],
    )?;
    run(repo.path(), &["config", "user.name", "Codexy Test"])?;
    write(repo.path(), path, &source)?;
    run(repo.path(), &["add", "."])?;
    run(repo.path(), &["commit", "-qm", "initial"])?;
    Ok(repo)
}

fn write(root: &Path, path: &str, text: &str) -> std::io::Result<()> {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture file parent"))?;
    std::fs::write(path, text)
}

fn validate(root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", "HEAD"])
        .current_dir(root)
        .output()?)
}

fn run(root: &Path, args: &[&str]) -> TestResult {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        stderr(&output)
    );
    Ok(())
}

fn blank_line_source() -> String {
    format!("\n\n{}", regular_lines(250))
}

fn multiline_source() -> String {
    format!(
        "{}let summary = format!(\n    \"status\"\n);\n",
        regular_lines(249)
    )
}

fn mixed_token_collapse_source() -> String {
    format!(
        "{}let report = format!(\n    \"{{}}/{{}}\",\n    \"status\",\n    1 + 2,\n    true,\n    \"ok\",\n    3 * 4,\n    5 - 1,\n    6 / 2,\n);\n",
        regular_lines(241)
    )
}

fn regular_lines(count: usize) -> String {
    (0..count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

fn regular_lines_from(start: usize, count: usize) -> String {
    (start..start + count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
