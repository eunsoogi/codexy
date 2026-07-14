mod support;

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

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
    for (path, module, extracted_path) in [
        ("src/foo.rs", "helper", "src/foo/helper.rs"),
        ("src/foo/mod.rs", "extracted", "src/foo/extracted.rs"),
        ("src/bin/too_large.rs", "worker", "src/bin/worker.rs"),
        ("src/bin/foo/main.rs", "helper", "src/bin/foo/helper.rs"),
        ("examples/foo/main.rs", "helper", "examples/foo/helper.rs"),
        ("tests/foo/main.rs", "helper", "tests/foo/helper.rs"),
        ("benches/foo/main.rs", "helper", "benches/foo/helper.rs"),
        ("src/custom_bin.rs", "helper", "src/helper.rs"),
        ("src/custom_dot_bin.rs", "helper", "src/helper.rs"),
        ("src/custom_parent_bin.rs", "helper", "src/helper.rs"),
        ("crates/app/build.rs", "worker", "crates/app/worker.rs"),
        (
            "crates/app/tests/too_large.rs",
            "scenarios",
            "crates/app/tests/scenarios.rs",
        ),
        (
            "src/parser/tests/case.rs",
            "helper",
            "src/parser/tests/case/helper.rs",
        ),
        ("src/too_large.rs", "worker", "src/too_large/worker.rs"),
        ("src/main.rs", "helper", "src/helper.rs"),
        ("src/lib.rs", "helper", "src/helper.rs"),
        ("src/foo/main.rs", "helper", "src/foo/main/helper.rs"),
        ("src/foo/lib.rs", "helper", "src/foo/lib/helper.rs"),
    ] {
        assert_module_split(path, module, extracted_path)?;
    }
    let repo = fixture(
        "src/too_large.rs",
        format!(
            "{}fn duplicate() {{}}\nfn duplicate() {{}}\nfn duplicate() {{}}\n",
            regular_lines(249)
        ),
    )?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!("{}fn duplicate() {{}}\n", regular_lines(249)),
    )?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn assert_module_split(path: &str, module: &str, extracted_path: &str) -> TestResult {
    let repo = fixture(path, regular_lines(252))?;
    write(
        repo.path(),
        path,
        &format!("mod {module};\n{}", regular_lines(249)),
    )?;
    write(repo.path(), extracted_path, &regular_lines_from(249, 3))?;
    let output = validate(repo.path())?;
    assert!(
        output.status.success(),
        "{path} should remain eligible\nstderr:\n{}",
        stderr(&output)
    );
    Ok(())
}
#[test]
fn touched_loc_rejects_sibling_files_for_nested_module_declarations() -> TestResult {
    for (path, sibling) in [
        ("src/foo.rs", "src/helper.rs"),
        ("src/foo/mod.rs", "src/foo.rs"),
        ("src/foo/main.rs", "src/foo/helper.rs"),
        ("src/foo/lib.rs", "src/foo/helper.rs"),
        ("src/custom_escape.rs", "src/helper.rs"),
        (
            "crates/app/src/parser/tests/case.rs",
            "crates/app/src/parser/tests/helper.rs",
        ),
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
